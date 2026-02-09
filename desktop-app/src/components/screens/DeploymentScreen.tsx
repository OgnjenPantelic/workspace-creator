import { useMemo } from "react";
import { useWizard } from "../../hooks/useWizard";
import type { DeploymentStep } from "../../hooks/useDeployment";

export function DeploymentScreen() {
  const ctx = useWizard();
  const {
    deployment,
    error,
    startDeploymentWizard: onStartDeployment,
    confirmAndDeploy: onConfirmDeploy,
    cancelDeployment: onCancel,
    rollback: onRollback,
    resetToWelcome: onResetToWelcome,
    copyToClipboard,
    setScreen,
    goBack: onGoBack,
  } = ctx;
  const deploymentStep = deployment.deploymentStep;
  const setDeploymentStep = deployment.setDeploymentStep;
  const deploymentStatus = deployment.deploymentStatus;
  const showDetailedLogs = deployment.showDetailedLogs;
  const setShowDetailedLogs = deployment.setShowDetailedLogs;
  const isRollingBack = deployment.isRollingBack;
  const templatePath = deployment.templatePath;
  const onOpenTemplateFolder = deployment.openTemplateFolder;
  const onGoToUcConfig = () => setScreen("unity-catalog-config");
  const status = deploymentStatus;
  
  // Helper to detect if the error is "Catalog already exists"
  const getCatalogExistsError = (output: string | undefined): string | null => {
    if (!output) return null;
    const match = output.match(/Catalog ['"]([^'"]+)['"] already exists/);
    return match ? match[1] : null;
  };

  // Memoized helper to count resources from terraform output
  const resourceCounts = useMemo(() => {
    if (!status?.output) return null;
    
    const output = status.output;
    const creatingSet = new Set<string>();
    const createdSet = new Set<string>();
    const destroyingSet = new Set<string>();
    const destroyedSet = new Set<string>();
    
    const lines = output.split('\n');
    for (const line of lines) {
      const creatingMatch = line.match(/^([^:]+):\s*Creating\.\.\.$/);
      if (creatingMatch) {
        creatingSet.add(creatingMatch[1].trim());
      }
      
      const createdMatch = line.match(/^([^:]+):\s*Creation complete/);
      if (createdMatch) {
        createdSet.add(createdMatch[1].trim());
      }
      
      const destroyingMatch = line.match(/^([^:]+):\s*Destroying\.\.\.$/);
      if (destroyingMatch) {
        destroyingSet.add(destroyingMatch[1].trim());
      }
      
      const destroyedMatch = line.match(/^([^:]+):\s*Destruction complete/);
      if (destroyedMatch) {
        destroyedSet.add(destroyedMatch[1].trim());
      }
    }
    
    return {
      creating: creatingSet.size,
      created: createdSet.size,
      destroying: destroyingSet.size,
      destroyed: destroyedSet.size,
    };
  }, [status?.output]);

  // Memoized helper to parse Terraform outputs from the output string
  const parsedOutputs = useMemo(() => {
    if (!status?.output) return {};
    
    const outputs: Record<string, string> = {};
    const lines = status.output.split('\n');
    
    for (const line of lines) {
      const match = line.match(/^(\w+)\s*=\s*"([^"]*)"$/);
      if (match) {
        outputs[match[1]] = match[2];
      }
    }
    
    return outputs;
  }, [status?.output]);
  
  // User-friendly step descriptions - change based on rollback state
  const stepInfo: Record<DeploymentStep, { title: string; description: string; icon: string }> = {
    ready: { title: "Ready to Deploy", description: "Click below to start the deployment process.", icon: "🚀" },
    initializing: { title: "Preparing Environment", description: "Setting up Terraform and downloading required providers...", icon: "⚙️" },
    planning: { title: "Analyzing Changes", description: "Determining what resources will be created...", icon: "📋" },
    review: { title: "Review & Confirm", description: "Review the planned changes and confirm to proceed.", icon: "✅" },
    deploying: isRollingBack 
      ? { title: "Cleaning Up Resources", description: "Removing deployed resources. This may take several minutes...", icon: "🧹" }
      : { title: "Creating Workspace", description: "Deploying your Databricks workspace. This may take 10-15 minutes...", icon: "🔨" },
    complete: isRollingBack
      ? { title: "Cleanup Complete!", description: "All resources have been successfully removed.", icon: "✅" }
      : { title: "Deployment Complete!", description: "Your Databricks workspace has been successfully created.", icon: "🎉" },
    failed: { title: "Deployment Failed", description: "An error occurred. Review the logs below for details.", icon: "❌" },
  };

  const currentStepInfo = stepInfo[deploymentStep];
  const isWorking = deploymentStep === "initializing" || deploymentStep === "planning" || deploymentStep === "deploying";

  return (
    <div className="container">
      {!isWorking && deploymentStep !== "complete" && (
        <button className="back-btn" onClick={onGoBack}>
          ← Back
        </button>
      )}
      
      {/* Progress indicator */}
      <div className="wizard-progress">
        <div className={`wizard-step ${deploymentStep === "initializing" || deploymentStep === "planning" || deploymentStep === "review" || deploymentStep === "deploying" || deploymentStep === "complete" ? "active" : ""} ${deploymentStep === "failed" ? "failed" : ""}`}>
          <div className="wizard-step-number">1</div>
          <div className="wizard-step-label">Prepare</div>
        </div>
        <div className="wizard-connector" />
        <div className={`wizard-step ${deploymentStep === "review" || deploymentStep === "deploying" || deploymentStep === "complete" ? "active" : ""}`}>
          <div className="wizard-step-number">2</div>
          <div className="wizard-step-label">Review</div>
        </div>
        <div className="wizard-connector" />
        <div className={`wizard-step ${deploymentStep === "deploying" || deploymentStep === "complete" ? "active" : ""}`}>
          <div className="wizard-step-number">3</div>
          <div className="wizard-step-label">{isRollingBack ? "Cleanup" : "Deploy"}</div>
        </div>
      </div>

      <div className="deployment-status-card">
        {currentStepInfo.icon && <div className="status-icon">{currentStepInfo.icon}</div>}
        <h1>{currentStepInfo.title}</h1>
        <p className="subtitle">{currentStepInfo.description}</p>
        
        {isWorking && (
          <div className="progress-indicator">
            <span className="spinner large" />
            {deploymentStep === "deploying" && resourceCounts && (resourceCounts.creating > 0 || resourceCounts.destroying > 0) && (
              <div style={{ marginTop: "12px", fontSize: "14px", color: "#a6a6a6" }}>
                {isRollingBack 
                  ? `${resourceCounts.destroyed} of ${resourceCounts.destroying} resources removed`
                  : `${resourceCounts.created} of ${resourceCounts.creating} resources created`
                }
              </div>
            )}
          </div>
        )}
      </div>

      {error && <div className="alert alert-error">{error}</div>}

      {/* Show logs toggle and template folder - hide folder on complete since it's in summary */}
      <div className="status-controls" style={{ marginTop: "24px", justifyContent: deploymentStep === "complete" ? "center" : "space-between", alignItems: "center" }}>
        <label className="log-toggle">
          <input
            type="checkbox"
            checked={showDetailedLogs}
            onChange={(e) => setShowDetailedLogs(e.target.checked)}
          />
          Show detailed logs
        </label>
        {templatePath && deploymentStep !== "complete" && (
          <div style={{ fontSize: "12px", color: "#757575" }}>
            Deployment folder: <code style={{ marginRight: "8px" }}>{templatePath}</code>
            <button 
              onClick={onOpenTemplateFolder}
              style={{ 
                background: "none", 
                border: "none", 
                color: "#ff6b35", 
                cursor: "pointer",
                fontSize: "12px",
                textDecoration: "underline"
              }}
            >
              Open
            </button>
          </div>
        )}
      </div>

      {/* Output */}
      {showDetailedLogs && status?.output && (
        <div className="output expanded">
          {status.output}
        </div>
      )}

      {/* Actions based on step */}
      <div style={{ marginTop: "24px", display: "flex", gap: "12px", justifyContent: "center" }}>
        {isWorking && (
          <button className="btn btn-danger" onClick={onCancel}>
            Cancel
          </button>
        )}
        
        {deploymentStep === "review" && (
          <>
            <button className="btn btn-secondary" onClick={onGoBack}>
              Go Back & Edit
            </button>
            <button className="btn btn-large btn-success" onClick={onConfirmDeploy}>
              Confirm & Deploy →
            </button>
          </>
        )}
        
        {deploymentStep === "complete" && !isRollingBack && (() => {
          const workspaceUrl = parsedOutputs.workspace_url;
          const metastoreStatus = parsedOutputs.metastore_status;
          const metastoreId = parsedOutputs.metastore_id;
          
          return (
            <div style={{ 
              display: "flex", 
              flexDirection: "column", 
              alignItems: "center",
              width: "100%",
              maxWidth: "600px",
              margin: "0 auto"
            }}>
              {/* Workspace URL - Primary CTA */}
              {workspaceUrl && (
                <div style={{
                  background: "linear-gradient(135deg, #1a3a1a 0%, #0d2610 100%)",
                  border: "1px solid #2d5a2d",
                  borderRadius: "12px",
                  padding: "24px",
                  marginBottom: "20px",
                  width: "100%",
                  textAlign: "center"
                }}>
                  <div style={{ color: "#888", fontSize: "12px", marginBottom: "8px", textTransform: "uppercase", letterSpacing: "1px" }}>
                    Your Workspace
                  </div>
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: "8px", marginBottom: "16px" }}>
                    <a 
                      href={workspaceUrl.startsWith("http") ? workspaceUrl : `https://${workspaceUrl}`} 
                      target="_blank" 
                      rel="noopener noreferrer"
                      style={{ 
                        color: "#4ade80", 
                        fontSize: "16px", 
                        fontWeight: "500",
                        wordBreak: "break-all"
                      }}
                    >
                      {workspaceUrl}
                    </a>
                    <button 
                      onClick={() => copyToClipboard(workspaceUrl)}
                      title="Copy URL"
                      style={{ 
                        background: "transparent", 
                        border: "none", 
                        color: "#4ade80",
                        cursor: "pointer",
                        fontSize: "14px",
                        padding: "4px"
                      }}
                    >
                      📋
                    </button>
                  </div>
                  <a 
                    href={workspaceUrl.startsWith("http") ? workspaceUrl : `https://${workspaceUrl}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="btn btn-success btn-large"
                    style={{ textDecoration: "none", display: "inline-block" }}
                  >
                    Open Workspace →
                  </a>
                </div>
              )}

              {/* Unity Catalog Info */}
              {(metastoreStatus || metastoreId) && (
                <div style={{
                  background: "#1e1e1e",
                  border: "1px solid #333",
                  borderRadius: "8px",
                  padding: "16px 20px",
                  marginBottom: "20px",
                  width: "100%"
                }}>
                  <div style={{ 
                    display: "flex", 
                    alignItems: "center", 
                    gap: "12px",
                    flexWrap: "wrap"
                  }}>
                    <span style={{ fontSize: "20px" }}>📊</span>
                    <div style={{ flex: 1 }}>
                      <div style={{ color: "#e0e0e0", fontSize: "14px", fontWeight: "500" }}>
                        Unity Catalog
                      </div>
                      <div style={{ color: "#888", fontSize: "13px" }}>
                        {metastoreStatus || `Metastore: ${metastoreId}`}
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {/* Secondary Actions */}
              <div style={{ 
                display: "flex", 
                gap: "12px", 
                width: "100%",
                marginBottom: "20px"
              }}>
                <button 
                  className="btn btn-secondary" 
                  style={{ flex: 1 }}
                  onClick={() => onResetToWelcome()}
                >
                  Start New Deployment
                </button>
                {templatePath && (
                  <button 
                    className="btn btn-secondary" 
                    style={{ flex: 1 }}
                    onClick={onOpenTemplateFolder}
                  >
                    Open Folder
                  </button>
                )}
              </div>

              {/* Delete Option */}
              {status?.can_rollback && (
                <button 
                  className="btn" 
                  onClick={onRollback}
                  style={{ 
                    background: "transparent",
                    border: "1px solid #666",
                    color: "#888",
                    fontSize: "13px",
                    padding: "8px 16px"
                  }}
                >
                  Delete Workspace & Resources
                </button>
              )}
            </div>
          );
        })()}
        
        {deploymentStep === "complete" && isRollingBack && (
          <button className="btn" onClick={() => onResetToWelcome(true)}>
            Start New Deployment
          </button>
        )}
        
        {deploymentStep === "failed" && (
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", width: "100%" }}>
            {/* Check if this is a "Catalog already exists" error */}
            {(() => {
              const catalogName = getCatalogExistsError(status?.output);
              if (catalogName) {
                return (
                  <div style={{ 
                    textAlign: "center", 
                    marginBottom: "16px",
                    padding: "12px",
                    background: "var(--bg-tertiary)",
                    borderRadius: "8px",
                    width: "100%",
                    maxWidth: "500px"
                  }}>
                    <p style={{ margin: "0 0 8px 0", color: "var(--text-secondary)" }}>
                      A catalog named <code style={{ color: "var(--text-primary)" }}>{catalogName}</code> already exists 
                      with a different storage location. Go back to change the catalog name or disable Unity Catalog.
                    </p>
                    <button 
                      className="btn btn-primary" 
                      style={{ marginTop: "8px" }}
                      onClick={() => {
                        onGoToUcConfig();
                        setDeploymentStep("ready");
                      }}
                    >
                      Change Configuration
                    </button>
                  </div>
                );
              }
              return null;
            })()}
            <div style={{ display: "flex", gap: "12px", justifyContent: "center" }}>
              <button className="btn" onClick={onStartDeployment}>
                Try Again
              </button>
              {status?.can_rollback && (
                <button className="btn btn-danger" onClick={onRollback}>
                  Cleanup Resources
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

import React from 'react';
import { CLOUDS, CLOUD_DISPLAY_NAMES } from '../../constants';
import { useWizard } from '../../hooks/useWizard';

const DependenciesScreen: React.FC = () => {
  const {
    dependencies,
    selectedCloud,
    error,
    installingTerraform,
    installTerraform,
    continueFromDependencies,
    goBack,
  } = useWizard();

  const terraformDep = dependencies["terraform"];
  const gitDep = dependencies["git"];
  const canContinue = terraformDep?.installed && gitDep?.installed;
  
  // Get the relevant cloud CLI based on selection
  const cloudCliKey = selectedCloud === CLOUDS.AWS ? "aws" : selectedCloud === CLOUDS.AZURE ? "azure" : "gcloud";
  const cloudCliDep = dependencies[cloudCliKey];
  const cloudCliName = selectedCloud === CLOUDS.AWS ? "AWS CLI" : selectedCloud === CLOUDS.AZURE ? "Azure CLI" : "Google Cloud CLI";
  
  // Databricks CLI - optional, enables profile-based auth
  const databricksCli = dependencies["databricks"];

  return (
    <div className="container">
      <button className="back-btn" onClick={goBack}>
        ← Back
      </button>
      <h1>System Requirements for {CLOUD_DISPLAY_NAMES[selectedCloud] || selectedCloud}</h1>
      <p className="subtitle">
        Let's make sure your system has everything needed for your {CLOUD_DISPLAY_NAMES[selectedCloud] || selectedCloud} deployment.
      </p>

      {error && <div className="alert alert-error">{error}</div>}

      <div className="dependency-list">
        {/* Terraform - Required */}
        <div className="dependency-item">
          <div className="dependency-info">
            <div className={`dependency-status ${terraformDep?.installed ? "installed" : "missing"}`} />
            <div>
              <div className="dependency-name">Terraform</div>
              {terraformDep?.version && (
                <div className="dependency-version">v{terraformDep.version}</div>
              )}
            </div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
            <span className="dependency-badge required">Required</span>
            {!terraformDep?.installed && (
              <button
                className="btn"
                onClick={installTerraform}
                disabled={installingTerraform}
              >
                {installingTerraform ? (
                  <>
                    <span className="spinner" />
                    Installing...
                  </>
                ) : (
                  "Install"
                )}
              </button>
            )}
          </div>
        </div>

        {/* Git - Required */}
        <div className="dependency-item">
          <div className="dependency-info">
            <div className={`dependency-status ${gitDep?.installed ? "installed" : "missing"}`} />
            <div>
              <div className="dependency-name">Git</div>
              {gitDep?.version && (
                <div className="dependency-version">{gitDep.version}</div>
              )}
            </div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
            <span className="dependency-badge required">Required</span>
            {!gitDep?.installed && (
              <a href={gitDep?.install_url} target="_blank" rel="noopener noreferrer" className="btn btn-secondary btn-small">
                Install Guide
              </a>
            )}
          </div>
        </div>

        {/* Cloud-specific CLI - Optional */}
        <div className="dependency-item">
          <div className="dependency-info">
            <div className={`dependency-status ${cloudCliDep?.installed ? "installed" : "optional"}`} />
            <div>
              <div className="dependency-name">{cloudCliName}</div>
              {cloudCliDep?.version && (
                <div className="dependency-version">{cloudCliDep.version}</div>
              )}
              {!cloudCliDep?.installed && (
                <div className="dependency-note">
                  {selectedCloud === CLOUDS.AWS 
                    ? "Enables profile-based authentication and SSO"
                    : selectedCloud === CLOUDS.AZURE 
                      ? "Enables interactive login and profile-based authentication"
                      : "Enables interactive login and profile-based authentication"}
                </div>
              )}
            </div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
            <span className="dependency-badge">Optional</span>
            {!cloudCliDep?.installed && (
              <a href={cloudCliDep?.install_url} target="_blank" className="btn btn-secondary btn-small">
                Install Guide
              </a>
            )}
          </div>
        </div>

        {/* Databricks CLI - Optional */}
        <div className="dependency-item">
          <div className="dependency-info">
            <div className={`dependency-status ${databricksCli?.installed ? "installed" : "optional"}`} />
            <div>
              <div className="dependency-name">Databricks CLI</div>
              {databricksCli?.version && (
                <div className="dependency-version">{databricksCli.version}</div>
              )}
              {!databricksCli?.installed && (
                <div className="help-text">Enables profile-based OAuth authentication</div>
              )}
            </div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
            <span className="dependency-badge">Optional</span>
            {!databricksCli?.installed && (
              <a href={databricksCli?.install_url} target="_blank" rel="noopener noreferrer" className="btn btn-secondary btn-small">
                Install Guide
              </a>
            )}
          </div>
        </div>
      </div>

      {!terraformDep?.installed && (
        <div className="alert alert-warning">
          Terraform is required to deploy workspaces. Click "Install" above to automatically
          download and install it, or install it manually from{" "}
          <a href={terraformDep?.install_url} target="_blank" rel="noopener noreferrer" style={{ color: "#ffb347" }}>
            terraform.io
          </a>
        </div>
      )}

      <div className="alert alert-info">
        <strong>Optional tools:</strong>
        <ul style={{ margin: "8px 0 0 0", paddingLeft: "20px", fontSize: "14px" }}>
          <li>
            <strong>{cloudCliName}</strong>: If installed, credentials will be auto-detected. 
            Otherwise, you can enter them manually in the next steps.
          </li>
          <li>
            <strong>Databricks CLI</strong>: If installed, enables profile-based OAuth authentication. 
            Otherwise, you can use service principal credentials.
          </li>
        </ul>
      </div>

      <div style={{ marginTop: "32px" }}>
        <button className="btn" onClick={continueFromDependencies} disabled={!canContinue}>
          Continue →
        </button>
      </div>
    </div>
  );
};

export default DependenciesScreen;

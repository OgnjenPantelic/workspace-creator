import React from 'react';
import { DependencyStatus } from '../../types';
import { CLOUDS, CLOUD_DISPLAY_NAMES } from '../../constants';

interface DependenciesScreenProps {
  dependencies: Record<string, DependencyStatus>;
  selectedCloud: string;
  error: string | null;
  installingTerraform: boolean;
  onInstallTerraform: () => void;
  onContinue: () => void;
  onBack: () => void;
}

const DependenciesScreen: React.FC<DependenciesScreenProps> = ({
  dependencies,
  selectedCloud,
  error,
  installingTerraform,
  onInstallTerraform,
  onContinue,
  onBack,
}) => {
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
      <button className="back-btn" onClick={onBack}>
        ← Back
      </button>
      <h1>System Requirements for {CLOUD_DISPLAY_NAMES[selectedCloud] || selectedCloud}</h1>
      <p className="subtitle">
        Let's make sure your system has everything needed for your {CLOUD_DISPLAY_NAMES[selectedCloud] || selectedCloud} deployment.
      </p>

      {error && <div className="alert alert-error">{error}</div>}

      <div className="dependencies-list">
        {/* Required Dependencies */}
        <div className={`dependency ${terraformDep?.installed ? "installed" : "missing"}`}>
          <div className="dep-info">
            <span className="dep-name">Terraform</span>
            <span className="dep-version">
              {terraformDep?.version || "Not installed"}
            </span>
            <span className="dep-badge required">Required</span>
          </div>
          {!terraformDep?.installed && (
            <button
              className="btn btn-small"
              onClick={onInstallTerraform}
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

        <div className={`dependency ${gitDep?.installed ? "installed" : "missing"}`}>
          <div className="dep-info">
            <span className="dep-name">Git</span>
            <span className="dep-version">
              {gitDep?.version || "Not installed"}
            </span>
            <span className="dep-badge required">Required</span>
          </div>
          {!gitDep?.installed && (
            <a
              href={gitDep?.install_url}
              target="_blank"
              rel="noopener noreferrer"
              className="btn btn-small"
            >
              Install Guide
            </a>
          )}
        </div>

        {/* Optional Cloud CLI */}
        <div className={`dependency ${cloudCliDep?.installed ? "installed" : "optional"}`}>
          <div className="dep-info">
            <span className="dep-name">{cloudCliName}</span>
            <span className="dep-version">
              {cloudCliDep?.version || "Not installed"}
            </span>
            <span className="dep-badge optional">Optional</span>
          </div>
          {!cloudCliDep?.installed && (
            <a
              href={cloudCliDep?.install_url}
              target="_blank"
              rel="noopener noreferrer"
              className="btn btn-small btn-secondary"
            >
              Install Guide
            </a>
          )}
        </div>
        {!cloudCliDep?.installed && (
          <p className="help-text" style={{ marginTop: "-8px", marginBottom: "12px" }}>
            {selectedCloud === CLOUDS.AWS 
              ? "AWS CLI enables profile-based authentication and SSO. Without it, you'll need to provide access keys manually."
              : selectedCloud === CLOUDS.AZURE 
                ? "Azure CLI enables interactive login and subscription management. Without it, you'll need to provide service principal credentials."
                : "GCloud CLI enables interactive login. Without it, you'll need to provide service account credentials."}
          </p>
        )}

        {/* Optional Databricks CLI */}
        <div className={`dependency ${databricksCli?.installed ? "installed" : "optional"}`}>
          <div className="dep-info">
            <span className="dep-name">Databricks CLI</span>
            <span className="dep-version">
              {databricksCli?.version || "Not installed"}
            </span>
            <span className="dep-badge optional">Optional</span>
          </div>
          {!databricksCli?.installed && (
            <a
              href={databricksCli?.install_url}
              target="_blank"
              rel="noopener noreferrer"
              className="btn btn-small btn-secondary"
            >
              Install Guide
            </a>
          )}
        </div>
        {!databricksCli?.installed && (
          <p className="help-text" style={{ marginTop: "-8px", marginBottom: "12px" }}>
            Databricks CLI enables profile-based authentication with OAuth. Without it, you'll need to provide service principal credentials.
          </p>
        )}
      </div>

      <div className="alert alert-info">
        <strong>{cloudCliName}</strong> is optional. If installed, credentials will be auto-detected.
        Otherwise, you can enter them manually.
        <br /><br />
        <strong>Databricks CLI</strong> is optional. If installed, you can authenticate using saved profiles
        or OAuth login. Otherwise, you can enter service principal credentials manually.
      </div>

      <div style={{ marginTop: "32px" }}>
        <button className="btn" onClick={onContinue} disabled={!canContinue}>
          Continue →
        </button>
      </div>
    </div>
  );
};

export default DependenciesScreen;

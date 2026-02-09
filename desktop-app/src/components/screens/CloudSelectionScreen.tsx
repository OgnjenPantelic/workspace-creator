import React from 'react';
import { CLOUDS, CLOUD_DISPLAY_NAMES } from '../../constants';
import { useWizard } from '../../hooks/useWizard';

const CloudSelectionScreen: React.FC = () => {
  const { loadingCloud, selectCloud, goBack } = useWizard();

  return (
    <div className="container">
      <button className="back-btn" onClick={goBack} disabled={!!loadingCloud}>
        ← Back
      </button>
      <h1>Select Cloud Provider</h1>
      <p className="subtitle">
        Select the cloud platform on which you would like to deploy your Databricks workspace.
      </p>

      {loadingCloud && (
        <div className="loading-overlay">
          <div className="loading-content">
            <span className="spinner large" />
            <div>Checking {CLOUD_DISPLAY_NAMES[loadingCloud] || loadingCloud} dependencies...</div>
          </div>
        </div>
      )}

      <div className="cloud-selection">
        <div 
          className={`cloud-card azure ${loadingCloud ? "disabled" : ""}`}
          onClick={() => !loadingCloud && selectCloud(CLOUDS.AZURE)}
        >
          <div className="cloud-name">Azure</div>
          <div className="cloud-description">
            Deploy on Microsoft Azure with VNet injection and Unity Catalog support
          </div>
        </div>

        <div 
          className={`cloud-card aws ${loadingCloud ? "disabled" : ""}`}
          onClick={() => !loadingCloud && selectCloud(CLOUDS.AWS)}
        >
          <div className="cloud-name">AWS</div>
          <div className="cloud-description">
            Deploy on Amazon Web Services with customer-managed VPC and Unity Catalog support
          </div>
        </div>

        <div 
          className={`cloud-card gcp ${loadingCloud ? "disabled" : ""}`}
          onClick={() => !loadingCloud && selectCloud(CLOUDS.GCP)}
        >
          <div className="cloud-name">GCP</div>
          <div className="cloud-description">
            Deploy on Google Cloud Platform with customer-managed VPC and Unity Catalog support
          </div>
        </div>
      </div>
    </div>
  );
};

export default CloudSelectionScreen;

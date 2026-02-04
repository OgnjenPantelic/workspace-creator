import React from 'react';
import { CLOUDS, CLOUD_DISPLAY_NAMES } from '../../constants';

interface CloudSelectionScreenProps {
  loadingCloud: string | null;
  onSelectCloud: (cloud: string) => void;
  onBack: () => void;
}

const CloudSelectionScreen: React.FC<CloudSelectionScreenProps> = ({
  loadingCloud,
  onSelectCloud,
  onBack,
}) => {
  return (
    <div className="container">
      <button className="back-btn" onClick={onBack} disabled={!!loadingCloud}>
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
          onClick={() => !loadingCloud && onSelectCloud(CLOUDS.AZURE)}
        >
          <div className="cloud-name">Azure</div>
          <div className="cloud-description">
            Deploy on Microsoft Azure with VNet injection and Unity Catalog support
          </div>
        </div>

        <div 
          className={`cloud-card aws ${loadingCloud ? "disabled" : ""}`}
          onClick={() => !loadingCloud && onSelectCloud(CLOUDS.AWS)}
        >
          <div className="cloud-name">AWS</div>
          <div className="cloud-description">
            Deploy on Amazon Web Services with customer-managed VPC and Unity Catalog support
          </div>
        </div>

        <div 
          className={`cloud-card gcp ${loadingCloud ? "disabled" : ""}`}
          onClick={() => !loadingCloud && onSelectCloud(CLOUDS.GCP)}
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

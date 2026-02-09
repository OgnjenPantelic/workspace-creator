import { CLOUDS } from "../../constants";
import { useWizard } from "../../hooks/useWizard";

export function TemplateSelectionScreen() {
  const { templates, selectedCloud, selectTemplate, goBack } = useWizard();
  const cloudTemplates = templates.filter((t) => t.cloud === selectedCloud);

  return (
    <div className="container">
      <button className="back-btn" onClick={goBack}>
        ← Back
      </button>
      <h1>Select Template</h1>
      <p className="subtitle">
        Choose the security and networking configuration that best fits your requirements.
      </p>

      <div className="templates">
        {cloudTemplates.length === 0 ? (
          <div className="template-card" style={{ opacity: 0.6, cursor: "not-allowed" }}>
            <div className="coming-soon">Coming Soon</div>
            <div className="template-title">
              {selectedCloud === CLOUDS.GCP ? "GCP Standard Workspace" : "Standard Workspace"}
            </div>
            <div className="template-description">
              Templates for this cloud provider are not yet available.
            </div>
          </div>
        ) : (
          <>
            {cloudTemplates.map((template) => (
              <div
                key={template.id}
                className="template-card"
                onClick={() => selectTemplate(template)}
              >
                <div className="template-title">{template.name}</div>
                <div className="template-description">{template.description}</div>
                <div className="template-features">
                  <ul>
                    {template.features.map((feature, i) => (
                      <li key={i}>{feature}</li>
                    ))}
                  </ul>
                </div>
              </div>
            ))}

            <div className="template-card" style={{ opacity: 0.6, cursor: "not-allowed" }}>
              <div className="coming-soon">Coming Soon</div>
              <div className="template-title">
                Maximum Security {selectedCloud === CLOUDS.AWS ? "PrivateLink" : ""} Workspace
              </div>
              <div className="template-description">
                Enterprise-grade security with {selectedCloud === CLOUDS.AWS ? "AWS PrivateLink" : "Private Link"} and zero internet exposure
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

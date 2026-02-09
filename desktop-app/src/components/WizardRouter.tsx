import { useWizard } from "../hooks/useWizard";
import {
  WelcomeScreen,
  CloudSelectionScreen,
  DependenciesScreen,
  TemplateSelectionScreen,
  ConfigurationScreen,
  UnityCatalogConfigScreen,
  DeploymentScreen,
  AwsCredentialsScreen,
  AzureCredentialsScreen,
  GcpCredentialsScreen,
  DatabricksCredentialsScreen,
} from "./screens";

export function WizardRouter() {
  const { screen } = useWizard();

  switch (screen) {
    case "welcome":
      return <WelcomeScreen />;
    case "cloud-selection":
      return <CloudSelectionScreen />;
    case "dependencies":
      return <DependenciesScreen />;
    case "aws-credentials":
      return <AwsCredentialsScreen />;
    case "azure-credentials":
      return <AzureCredentialsScreen />;
    case "gcp-credentials":
      return <GcpCredentialsScreen />;
    case "databricks-credentials":
      return <DatabricksCredentialsScreen />;
    case "template-selection":
      return <TemplateSelectionScreen />;
    case "configuration":
      return <ConfigurationScreen />;
    case "unity-catalog-config":
      return <UnityCatalogConfigScreen />;
    case "deployment":
      return <DeploymentScreen />;
    default:
      return <WelcomeScreen />;
  }
}

export const ASSISTANT = {
  MAX_HISTORY_MESSAGES: 20,
} as const;

export const ASSISTANT_PROVIDERS = {
  "github-models": {
    name: "GitHub Models",
    description: "Free AI models from GitHub",
    apiKeyUrl: "https://github.com/settings/personal-access-tokens/new",
    apiKeyPlaceholder: "github_pat_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    instructions: "Create a Fine-grained Personal Access Token with 'models:read' permission (Account permissions → Models → Read-only)",
    recommended: true,
  },
  "openai": {
    name: "OpenAI",
    description: "GPT-4o mini (paid)",
    apiKeyUrl: "https://platform.openai.com/api-keys",
    apiKeyPlaceholder: "sk-proj-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    instructions: "Create a new API key from your OpenAI dashboard",
    recommended: false,
  },
  "claude": {
    name: "Claude",
    description: "Claude 3.5 Haiku (paid)",
    apiKeyUrl: "https://console.anthropic.com/settings/keys",
    apiKeyPlaceholder: "sk-ant-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    instructions: "Create a new API key from the Anthropic console",
    recommended: false,
  },
} as const;

export const SCREEN_CONTEXT: Record<string, string> = {
  "welcome": "The user is on the welcome screen. They haven't started any configuration yet. They can click 'Get Started' to begin the deployment wizard.",
  "cloud-selection": "The user is choosing a cloud provider: AWS, Azure, or GCP. Each card shows supported features. They click a cloud to proceed.",
  "dependencies": "The user is on the dependencies screen where the app checks if Terraform CLI and Databricks CLI are installed. Terraform can be auto-installed. Databricks CLI is optional but recommended.",
  "aws-credentials": "The user is configuring AWS credentials. Two modes: 'AWS CLI Profile' (recommended, uses ~/.aws/credentials or ~/.aws/config, supports SSO) or 'Access Keys' (manual key entry). The app verifies identity and checks IAM permissions.",
  "azure-credentials": "The user is configuring Azure credentials. Two modes: 'Azure CLI' (recommended, uses 'az login') or 'Service Principal' (Tenant ID, Subscription ID, Client ID, Client Secret). After auth, they select a subscription and the app checks role assignments.",
  "gcp-credentials": "The user is configuring GCP credentials. Two modes: 'Application Default Credentials' (recommended, uses gcloud + service account impersonation) or 'Service Account Key' (paste JSON key). The service account needs Owner role on the project.",
  "databricks-credentials": "The user is entering Databricks account credentials. For GCP/Azure-identity: just the Account ID. For AWS/Azure-SP: either a CLI profile from ~/.databrickscfg (service principal only) or Client ID + Client Secret. The Account ID is a UUID from the Databricks Account Console.",
  "template-selection": "The user is selecting a Terraform deployment template. Currently one template per cloud (aws-simple, azure-simple, gcp-simple). Each shows what infrastructure it creates.",
  "configuration": "The user is filling in Terraform template variables: workspace name, region, networking (VPC/VNet/subnet CIDRs), tags, and optional advanced settings like existing VPC/VNet. Values have validation rules.",
  "unity-catalog-config": "The user is configuring Unity Catalog (optional). They can enable it with a catalog name and storage location (S3 bucket/Azure Storage/GCS bucket). The app auto-detects if a metastore exists in the region. Storage names must be globally unique.",
  "deployment": "The user is on the deployment screen. Terraform runs in stages: init → plan → review → apply. They can see real-time output, review the plan before applying, cancel a running deployment, or rollback after failure. Deployment typically takes 5-15 minutes.",
};

export const ASSISTANT_SAMPLE_QUESTIONS: Record<string, string[]> = {
  "welcome": [
    "What does this app do?",
    "Which cloud provider should I choose?",
    "What prerequisites do I need?",
  ],
  "cloud-selection": [
    "What's the difference between AWS, Azure, and GCP deployment?",
    "Can I deploy to multiple clouds?",
  ],
  "dependencies": [
    "Is Databricks CLI required?",
    "Can the app auto-install dependencies?",
  ],
  "aws-credentials": [
    "Should I use SSO or access keys?",
    "Where do I find my AWS profile?",
    "What IAM permissions are needed?",
  ],
  "azure-credentials": [
    "What's the difference between Azure CLI and Service Principal?",
    "How do I run az login?",
    "What Azure roles do I need?",
  ],
  "gcp-credentials": [
    "How does service account impersonation work?",
    "What GCP permissions are required?",
    "Where do I get the service account JSON key?",
  ],
  "databricks-credentials": [
    "Where do I find my Databricks Account ID?",
    "What's the difference between OAuth and service principal?",
    "Can I use my existing databrickscfg profile?",
  ],
  "template-selection": [
    "What does the templates create?",
    "Can I use an existing VPC?",
    "What's the difference between templates?",
  ],
  "configuration": [
    "What CIDR ranges should I use?",
    "How do I configure VPC settings?",
  ],
  "unity-catalog-config": [
    "What is Unity Catalog?",
    "Do I need to create a new metastore?",
    "How do I choose a storage location?",
  ],
  "deployment": [
    "What happens during the plan stage?",
    "Can I cancel a running deployment?",
  ],
};

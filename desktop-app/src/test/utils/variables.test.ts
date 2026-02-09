import {
  formatVariableName,
  groupVariablesBySection,
  generateRandomSuffix,
  initializeFormDefaults,
} from "../../utils/variables";
import { TerraformVariable } from "../../types";
import { DEFAULTS } from "../../constants";

// Helper to create a minimal TerraformVariable
function makeVar(
  name: string,
  defaults?: Partial<TerraformVariable>
): TerraformVariable {
  return {
    name,
    description: defaults?.description ?? "",
    var_type: defaults?.var_type ?? "string",
    default: defaults?.default ?? null,
    required: defaults?.required ?? false,
    sensitive: defaults?.sensitive ?? false,
    validation: defaults?.validation ?? null,
  };
}

// ---------------------------------------------------------------------------
// formatVariableName
// ---------------------------------------------------------------------------
describe("formatVariableName", () => {
  it("returns a constant display name when one exists", () => {
    expect(formatVariableName("prefix")).toBe("Workspace Name");
    expect(formatVariableName("location")).toBe("Region");
    expect(formatVariableName("admin_user")).toBe("Admin Email");
  });

  it("converts snake_case to Title Case for unknown variables", () => {
    expect(formatVariableName("my_custom_var")).toBe("My Custom Var");
  });

  it("handles a single word", () => {
    expect(formatVariableName("foobar")).toBe("Foobar");
  });

  it("handles an empty string", () => {
    expect(formatVariableName("")).toBe("");
  });
});

// ---------------------------------------------------------------------------
// groupVariablesBySection
// ---------------------------------------------------------------------------
describe("groupVariablesBySection", () => {
  it("groups known variables into the correct sections", () => {
    const vars = [makeVar("prefix"), makeVar("region"), makeVar("vpc_cidr_range")];
    const sections = groupVariablesBySection(vars);

    expect(sections["Workspace"]).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ name: "prefix" }),
        expect.objectContaining({ name: "region" }),
      ])
    );
    expect(sections["Advanced: Network Configuration"]).toEqual(
      expect.arrayContaining([expect.objectContaining({ name: "vpc_cidr_range" })])
    );
  });

  it("puts unknown variables into 'Other Configuration'", () => {
    const vars = [makeVar("some_unknown_var")];
    const sections = groupVariablesBySection(vars);

    expect(sections["Other Configuration"]).toHaveLength(1);
    expect(sections["Other Configuration"][0].name).toBe("some_unknown_var");
  });

  it("excludes variables in EXCLUDE_VARIABLES", () => {
    const vars = [
      makeVar("databricks_account_id"),
      makeVar("aws_access_key_id"),
      makeVar("prefix"),
    ];
    const sections = groupVariablesBySection(vars);

    // Only prefix should appear
    const allVarNames = Object.values(sections)
      .flat()
      .map((v) => v.name);

    expect(allVarNames).toContain("prefix");
    expect(allVarNames).not.toContain("databricks_account_id");
    expect(allVarNames).not.toContain("aws_access_key_id");
  });

  it("returns an empty object for empty input", () => {
    expect(groupVariablesBySection([])).toEqual({});
  });

  it("handles a mix of known, unknown, and excluded variables", () => {
    const vars = [
      makeVar("prefix"),                  // known: Workspace
      makeVar("gcp_project_id"),          // excluded
      makeVar("my_custom_thing"),         // unknown: Other Configuration
      makeVar("cidr_block"),              // known: Advanced: Network Configuration
    ];
    const sections = groupVariablesBySection(vars);

    const allVarNames = Object.values(sections).flat().map((v) => v.name);
    expect(allVarNames).toEqual(
      expect.arrayContaining(["prefix", "my_custom_thing", "cidr_block"])
    );
    expect(allVarNames).not.toContain("gcp_project_id");
    expect(Object.keys(sections)).toEqual(
      expect.arrayContaining([
        "Workspace",
        "Advanced: Network Configuration",
        "Other Configuration",
      ])
    );
  });
});

// ---------------------------------------------------------------------------
// generateRandomSuffix
// ---------------------------------------------------------------------------
describe("generateRandomSuffix", () => {
  it("returns a string of length 6", () => {
    expect(generateRandomSuffix()).toHaveLength(6);
  });

  it("returns only lowercase alphanumeric characters", () => {
    for (let i = 0; i < 20; i++) {
      expect(generateRandomSuffix()).toMatch(/^[a-z0-9]{6}$/);
    }
  });

  it("returns different values on consecutive calls", () => {
    const results = new Set(Array.from({ length: 50 }, () => generateRandomSuffix()));
    // With 36^6 possibilities, 50 calls should produce at least 2 unique values
    expect(results.size).toBeGreaterThan(1);
  });
});

// ---------------------------------------------------------------------------
// initializeFormDefaults
// ---------------------------------------------------------------------------
describe("initializeFormDefaults", () => {
  it("sets prefix to '{default}-{randomSuffix}' pattern", () => {
    const vars = [makeVar("prefix", { default: "databricks" })];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.prefix).toMatch(/^databricks-[a-z0-9]{6}$/);
  });

  it("uses 'databricks' as base prefix when no default", () => {
    const vars = [makeVar("prefix")];
    const defaults = initializeFormDefaults(vars);

    // default is null → falsy → fallback to "databricks"
    // Actually: `v.default || "databricks"` — null is falsy, so falls back
    expect(defaults.prefix).toMatch(/^databricks-[a-z0-9]{6}$/);
  });

  it("sets workspace_name to 'databricks-ws-{randomSuffix}'", () => {
    const vars = [makeVar("workspace_name")];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.workspace_name).toMatch(/^databricks-ws-[a-z0-9]{6}$/);
  });

  it("sets databricks_workspace_name the same as workspace_name", () => {
    const vars = [makeVar("databricks_workspace_name")];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.databricks_workspace_name).toMatch(/^databricks-ws-[a-z0-9]{6}$/);
  });

  it("sets root_storage_name to 'dbstorage{shortSuffix}' without hyphens", () => {
    const vars = [makeVar("root_storage_name")];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.root_storage_name).toMatch(/^dbstorage[a-z0-9]+$/);
    expect(defaults.root_storage_name).not.toContain("-");
  });

  it("sets vnet_name to empty string (filled by user when using existing VNet)", () => {
    const vars = [makeVar("vnet_name")];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.vnet_name).toBe("");
  });

  it("sets vnet_resource_group_name to empty string", () => {
    const vars = [makeVar("vnet_resource_group_name")];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.vnet_resource_group_name).toBe("");
  });

  it("sets subnet CIDRs to DEFAULTS values", () => {
    const vars = [makeVar("subnet_public_cidr"), makeVar("subnet_private_cidr")];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.subnet_public_cidr).toBe(DEFAULTS.PUBLIC_SUBNET_CIDR);
    expect(defaults.subnet_private_cidr).toBe(DEFAULTS.PRIVATE_SUBNET_CIDR);
  });

  it("sets location to DEFAULTS.AZURE_REGION", () => {
    const vars = [makeVar("location")];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.location).toBe(DEFAULTS.AZURE_REGION);
  });

  it("sets google_region to variable default if present", () => {
    const vars = [makeVar("google_region", { default: "europe-west1" })];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.google_region).toBe("europe-west1");
  });

  it("sets google_region to DEFAULTS.GCP_REGION when no default", () => {
    const vars = [makeVar("google_region")];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.google_region).toBe(DEFAULTS.GCP_REGION);
  });

  it("sets admin_user from context.azureUser when provided", () => {
    const vars = [makeVar("admin_user")];
    const defaults = initializeFormDefaults(vars, { azureUser: "azure@test.com" });

    expect(defaults.admin_user).toBe("azure@test.com");
  });

  it("sets admin_user from context.gcpAccount when provided", () => {
    const vars = [makeVar("admin_user")];
    const defaults = initializeFormDefaults(vars, { gcpAccount: "gcp@test.com" });

    expect(defaults.admin_user).toBe("gcp@test.com");
  });

  it("sets create_new_resource_group to true", () => {
    const vars = [makeVar("create_new_resource_group")];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.create_new_resource_group).toBe(true);
  });

  it("uses the variable's own default when present for generic variables", () => {
    const vars = [makeVar("some_var", { default: "my_default" })];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.some_var).toBe("my_default");
  });

  it("falls back to empty string when no default exists", () => {
    const vars = [makeVar("some_var")];
    const defaults = initializeFormDefaults(vars);

    expect(defaults.some_var).toBe("");
  });
});

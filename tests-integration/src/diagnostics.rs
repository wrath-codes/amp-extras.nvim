//! Integration tests for diagnostics collection

use amp_extras_core as amp_extras;
use nvim_oxi::api;
use serde_json::json;

#[nvim_oxi::test]
fn test_diagnostics_with_errors() {
    amp_extras::ide_ops::mark_nvim_ready();
    // Create buffer with explicit name
    let ns = api::create_namespace("amp_extras_tests");
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/amp-diag-errors.rs").unwrap();
    buf.set_lines(.., false, ["fn main() {", "  let x = 1", "}"])
        .unwrap();

    // Set diagnostic via Lua with namespace using actual buffer number
    let bufnr = buf.handle();
    let lua_expr = format!("vim.diagnostic.set({}, {}, {{ {{ lnum = 0, col = 0, end_lnum = 0, end_col = 10, severity = vim.diagnostic.severity.ERROR, message = 'test error' }} }})", ns, bufnr);
    api::call_function::<_, ()>("luaeval", (lua_expr,)).unwrap();

    // Call our diagnostics function
    let result = amp_extras::ide_ops::get_diagnostics(json!({}));

    eprintln!("get_diagnostics result: {:?}", result);

    let result = result.unwrap();
    eprintln!(
        "Diagnostics result JSON: {}",
        serde_json::to_string_pretty(&result).unwrap()
    );

    // Assertions
    let entries = result["entries"].as_array().unwrap();
    eprintln!("Entries count: {}", entries.len());
    assert!(!entries.is_empty(), "Should have diagnostics");

    let first_entry = &entries[0];
    let diags = first_entry["diagnostics"].as_array().unwrap();
    assert_eq!(diags[0]["severity"], "ERROR");
    assert_eq!(diags[0]["description"], "test error");
}

#[nvim_oxi::test]
fn test_diagnostics_filters_by_path() {
    amp_extras::ide_ops::mark_nvim_ready();
    // Create buffer with explicit name
    let ns = api::create_namespace("amp_test_filter_path");
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/amp-test-filter.rs").unwrap();
    buf.set_lines(.., false, ["test content"]).unwrap();

    // Set diagnostic with namespace using actual buffer number
    let bufnr = buf.handle();
    let lua_expr = format!(
        "vim.diagnostic.set({}, {}, {{ {{ lnum = 0, col = 0, message = 'test diagnostic' }} }})",
        ns, bufnr
    );
    api::call_function::<_, ()>("luaeval", (lua_expr,)).unwrap();

    // Get all diagnostics first (no filter)
    let result = amp_extras::ide_ops::get_diagnostics(json!({})).unwrap();
    let entries = result["entries"].as_array().unwrap();
    assert!(
        !entries.is_empty(),
        "Should find diagnostics without filter"
    );

    // Get the actual buffer path from the result
    let actual_uri = entries[0]["uri"].as_str().unwrap();

    // Now filter by a prefix that should match
    // Use /private/tmp instead of /tmp to avoid symlink issues on macOS
    let result = amp_extras::ide_ops::get_diagnostics(json!({ "path": "/private/tmp" })).unwrap();
    assert!(
        !result["entries"].as_array().unwrap().is_empty(),
        "Should find diagnostics with /private/tmp prefix, actual uri: {}",
        actual_uri
    );

    // Different prefix - should be empty
    let result = amp_extras::ide_ops::get_diagnostics(json!({ "path": "/other" })).unwrap();
    assert!(
        result["entries"].as_array().unwrap().is_empty(),
        "Should not find diagnostics with non-matching prefix"
    );
}

#[nvim_oxi::test]
fn test_diagnostics_multiple_buffers() {
    amp_extras::ide_ops::mark_nvim_ready();
    let ns = api::create_namespace("amp_extras_tests");

    // Create first buffer with diagnostic
    let mut buf1 = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf1).unwrap();
    buf1.set_name("/tmp/amp-test1.rs").unwrap();
    buf1.set_lines(.., false, ["let x = 1;"]).unwrap();

    let bufnr1 = buf1.handle();
    let lua_expr = format!("vim.diagnostic.set({}, {}, {{ {{ lnum = 0, col = 0, message = 'error in file 1', severity = vim.diagnostic.severity.ERROR }} }})", ns, bufnr1);
    api::call_function::<_, ()>("luaeval", (lua_expr,)).unwrap();

    // Create second buffer with diagnostic
    let mut buf2 = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf2).unwrap();
    buf2.set_name("/tmp/amp-test2.rs").unwrap();
    buf2.set_lines(.., false, ["let y = 2;"]).unwrap();

    let bufnr2 = buf2.handle();
    let lua_expr = format!("vim.diagnostic.set({}, {}, {{ {{ lnum = 0, col = 0, message = 'warning in file 2', severity = vim.diagnostic.severity.WARN }} }})", ns, bufnr2);
    api::call_function::<_, ()>("luaeval", (lua_expr,)).unwrap();

    // Get all diagnostics
    let result = amp_extras::ide_ops::get_diagnostics(json!({})).unwrap();
    let entries = result["entries"].as_array().unwrap();

    // Should have diagnostics from both buffers
    assert!(entries.len() >= 2, "Should collect from multiple buffers");
}

#[nvim_oxi::test]
fn test_diagnostics_severity_mapping() {
    amp_extras::ide_ops::mark_nvim_ready();
    let ns = api::create_namespace("amp_extras_tests");
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/amp-severity.rs").unwrap();
    buf.set_lines(.., false, ["test"]).unwrap();

    // Test all severity levels using actual buffer number
    let bufnr = buf.handle();
    let lua_expr = format!("vim.diagnostic.set({}, {}, {{ {{ lnum = 0, col = 0, message = 'error', severity = vim.diagnostic.severity.ERROR }}, {{ lnum = 0, col = 1, message = 'warning', severity = vim.diagnostic.severity.WARN }}, {{ lnum = 0, col = 2, message = 'info', severity = vim.diagnostic.severity.INFO }}, {{ lnum = 0, col = 3, message = 'hint', severity = vim.diagnostic.severity.HINT }} }})", ns, bufnr);
    api::call_function::<_, ()>("luaeval", (lua_expr,)).unwrap();

    let result = amp_extras::ide_ops::get_diagnostics(json!({})).unwrap();
    let entries = result["entries"].as_array().unwrap();
    let diags = entries[0]["diagnostics"].as_array().unwrap();

    // Verify severity mapping
    assert_eq!(diags[0]["severity"], "ERROR");
    assert_eq!(diags[1]["severity"], "WARNING");
    assert_eq!(diags[2]["severity"], "INFO");
    assert_eq!(diags[3]["severity"], "HINT");
}

#[nvim_oxi::test]
fn test_diagnostics_empty_when_none() {
    // Fresh buffer with no diagnostics
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/amp-clean.rs").unwrap();
    buf.set_lines(.., false, ["clean code"]).unwrap();

    let result = amp_extras::ide_ops::get_diagnostics(json!({})).unwrap();
    let entries = result["entries"].as_array().unwrap();

    // Should be empty when no diagnostics are set
    assert!(
        entries.is_empty(),
        "Should not include buffers without diagnostics"
    );
}

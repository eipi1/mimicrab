import pytest
from playwright.sync_api import Page, expect
import time

def test_lua_mock_creation_and_execution(page: Page):
    # Navigate to the dashboard
    page.goto("http://localhost:3000")
    
    # Click Create Mock
    page.click("#btn-create-mock")
    
    # Fill basic details
    page.fill("#mock-path", "/lua-ui-test")
    page.select_option("#mock-method", "POST")
    
    # Open Advanced Options
    page.click("#btn-toggle-advanced")
    time.sleep(1) # Wait for animation
    
    # Take screenshot for debugging if it fails here
    page.screenshot(path="debug_advanced.png")
    
    # Enable Lua Scripting - click the switch label as requested
    page.locator(".lua-control .switch").click()
    
    # Verify it is checked
    expect(page.locator("#mock-lua-enabled")).to_be_checked()
    
    # Verify Jitter and Proxy are disabled (optional check)
    expect(page.locator("#mock-jitter-enabled")).not_to_be_checked()
    expect(page.locator("#mock-proxy-enabled")).not_to_be_checked()
    
    # Enter Lua Script
    lua_script = """
local res = {
    status = 201,
    headers = { ["X-From-Lua"] = "UI-Test" },
    body = {
        msg = "Hello from Lua UI!",
        method = request.method
    }
}
return res
"""
    page.fill("#mock-lua-script", lua_script)
    
    # Save Mock
    page.click("button:has-text('Save Mock')")
    time.sleep(1) # Wait for list to refresh
    
    # Wait for the mock to appear in the list
    expect(page.locator(".mock-card-header:has-text('ID:')").first).to_be_visible()
    
    # Find our mock card and click Test
    # Since we only have one mock (or it's the newest), we can find it by path
    mock_card = page.locator(".card", has=page.locator(".mock-path", has_text="/lua-ui-test")).first
    mock_card.locator(".test-btn").click()
    
    # Verify Test Result Modal
    expect(page.locator("#test-modal")).to_be_visible()
    
    # Wait for response and verify content
    # Look for the response body in the test result
    test_result_body = page.locator("#test-res-body-value")
    expect(test_result_body).to_contain_text("Hello from Lua UI!")

    # Close Test Result
    page.click("#btn-close-test-modal")
    
    # Re-open Edit to verify persistence
    mock_card.locator(".edit-btn").click()
    page.click("#btn-toggle-advanced")
    expect(page.locator("#mock-lua-enabled")).to_be_checked()
    # Check if value matches (handling potential whitespace/indentation differences)
    actual_script = page.locator("#mock-lua-script").input_value()
    assert actual_script.strip() == lua_script.strip()
    
    # Close Modal
    page.click("#btn-close-modal")

if __name__ == "__main__":
    pytest.main([__file__])

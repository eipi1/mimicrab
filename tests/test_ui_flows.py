import pytest
import requests
from playwright.sync_api import Page, expect

BASE_URL = "http://localhost:3000"

@pytest.fixture(autouse=True)
def clean_mocks():
    """Ensure mocks are cleared before each test."""
    requests.post(f"{BASE_URL}/_admin/import", json=[])
    yield

def test_dashboard_full_flow(page: Page):
    # 1. Open Dashboard
    page.goto(BASE_URL)
    expect(page.get_by_role("heading", name="Mimicrab")).to_be_visible()
    
    # 2. Create a Mock
    page.get_by_role("button", name="+ Create Mock").click()
    page.get_by_label("Method").select_option("GET")
    page.locator("#mock-path").fill("/auto-test")
    page.locator("#mock-status").fill("202")
    page.locator("#mock-res-body").fill('{"auto": "ready"}')
    
    # Advanced Options
    page.click("#btn-toggle-advanced")
    expect(page.locator("#advanced-options")).to_be_visible()
    page.locator("#mock-latency").fill("100")
    
    # Jitter - interaction with styled switch
    page.locator("label.switch").click() # Click the slider container
    page.locator("#mock-jitter-prob").fill("10")
    page.locator("#mock-jitter-status").fill("500")
    page.locator("#mock-jitter-body").fill('{"error": "jitter"}')
    
    # Add a header
    page.get_by_role("button", name="+ Add Res Header").click()
    page.locator(".header-key").first.fill("X-Auto")
    page.locator(".header-value").first.fill("verified")
    
    page.get_by_role("button", name="Save Mock").click()
    
    # Verify card appeared
    expect(page.locator(".card").filter(has_text="/auto-test")).to_be_visible()
    expect(page.locator(".card").filter(has_text="Return 202")).to_be_visible()

    # 3. Use "Test" button
    page.get_by_role("button", name="Test").first.click()
    expect(page.locator("#test-modal")).to_be_visible()
    # Wait for result (avoid race condition with loading)
    expect(page.locator("#test-result-content")).to_contain_text("Status Code")
    expect(page.locator("#test-result-content")).to_contain_text("/auto-test")
    
    # Check curl command
    expect(page.locator("#curl-command")).to_contain_text("curl -X GET")
    expect(page.locator("#curl-command")).to_contain_text("/auto-test")
    
    page.get_by_role("button", name="Close").click()
    expect(page.locator("#test-modal")).not_to_be_visible()
    
    # 4. Check Logs
    page.get_by_role("button", name="Logs").click()
    expect(page.locator(".log-entry").first).to_contain_text("/auto-test")
    expect(page.locator(".log-entry").first).to_contain_text("MATCH")

    # 5. Export Mocks (Just click and check no error)
    page.get_by_role("button", name="Export/Import").click()
    page.get_by_role("button", name="Export to JSON").click()

    # 6. Edit Mock (This would have caught the Jitter TypeError)
    page.get_by_role("button", name="Mocks").click()
    page.get_by_role("button", name="Edit").first.click()
    expect(page.locator("#modal-title")).to_contain_text("Edit Mock")
    
    # Verify values are populated (hydration)
    expect(page.locator("#mock-path")).to_have_value("/auto-test")
    expect(page.locator("#mock-status")).to_have_value("202")
    
    # Check if jitter is still enabled in the form
    expect(page.locator("#mock-jitter-enabled")).to_be_checked()
    expect(page.locator("#mock-jitter-status")).to_have_value("500")
    
    # Modify something and save
    page.locator("#mock-status").fill("201")
    page.get_by_role("button", name="Save Mock").click()
    
    # Verify update
    expect(page.locator(".card").filter(has_text="Return 201")).to_be_visible()

    # 7. Delete Mock
    page.once("dialog", lambda dialog: dialog.accept()) # Handle confirm delete
    page.get_by_role("button", name="Delete").first.click()
    
    # Verify specific mock is gone
    expect(page.locator(".card").filter(has_text="/auto-test")).not_to_be_visible()

def test_non_json_response(page: Page):
    page.goto(BASE_URL)
    
    # Create a non-JSON mock
    page.get_by_role("button", name="+ Create Mock").click()
    page.locator("#mock-path").fill("/text-test")
    
    # Select Text type
    page.locator("#mock-body-type").select_option("text")
    page.locator("#mock-res-body").fill("Hello World <html>")
    
    page.get_by_role("button", name="Save Mock").click()
    
    # Test it
    page.get_by_role("button", name="Test").first.click()
    expect(page.locator("#test-result-content")).to_contain_text("Status Code")
    expect(page.locator("#test-result-content")).to_contain_text("Hello World <html>")
    
    # Verify curl for text body (no content-type json header if not specified)
    expect(page.locator("#curl-command")).to_contain_text("curl -X GET")
    
    page.get_by_role("button", name="Close").click()
    
    # Cleanup
    page.once("dialog", lambda dialog: dialog.accept())
    page.get_by_role("button", name="Delete").first.click()

def test_jitter_full_features(page: Page):
    page.goto(BASE_URL)
    
    # Create a mock with full jitter features
    page.get_by_role("button", name="+ Create Mock").click()
    page.locator("#mock-path").fill("/jitter-advanced")
    
    page.click("#btn-toggle-advanced")
    page.locator("label.switch").click() # Enable jitter
    
    page.locator("#mock-jitter-prob").fill("100") # Always trigger for test
    page.locator("#mock-jitter-status").fill("418")
    
    # Jitter header
    page.get_by_role("button", name="+ Add Jitter Header").click()
    page.locator("#jitter-settings .header-key").first.fill("X-Jitter-Type")
    page.locator("#jitter-settings .header-value").first.fill("intermittent")
    
    # Jitter body type and content
    page.locator("#mock-jitter-body-type").select_option("text")
    page.locator("#mock-jitter-body").fill("Jitter Error Page")
    page.locator("#mock-jitter-latency").fill("50")
    
    page.get_by_role("button", name="Save Mock").click()
    
    # Edit it to make sure hydration works for jitter headers/body/latency
    page.get_by_role("button", name="Edit").first.click()
    expect(page.locator("#mock-jitter-status")).to_have_value("418")
    expect(page.locator("#mock-jitter-latency")).to_have_value("50")
    expect(page.locator("#mock-jitter-body")).to_have_value("Jitter Error Page")
    expect(page.locator("#jitter-settings .header-key")).to_have_value("X-Jitter-Type")
    
    page.get_by_role("button", name="Save Mock").click()
    
    # Clean up
    page.once("dialog", lambda dialog: dialog.accept())
    page.get_by_role("button", name="Delete").first.click()

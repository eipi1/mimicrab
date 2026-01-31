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

    # 6. Delete Mock
    page.get_by_role("button", name="Mocks").click()
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

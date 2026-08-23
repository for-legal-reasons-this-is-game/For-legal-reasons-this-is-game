import requests
import base64
import sys

# Logto Core endpoint (The Management API lives here)
LOGTO_ENDPOINT = "http://localhost:3001"
RESOURCE_INDICATOR = "https://default.logto.app/api"


def get_management_token(client_id, client_secret):
    url = f"{LOGTO_ENDPOINT}/oidc/token"
    auth_string = f"{client_id}:{client_secret}"
    b64_auth = base64.b64encode(auth_string.encode()).decode()

    headers = {
        "Authorization": f"Basic {b64_auth}",
        "Content-Type": "application/x-www-form-urlencoded",
    }

    data = {
        "grant_type": "client_credentials",
        "resource": RESOURCE_INDICATOR,
        "scope": "all",
    }

    response = requests.post(url, headers=headers, data=data)

    if response.status_code != 200:
        print(f"Failed to get token: {response.text}")
        sys.exit(1)

    return response.json()["access_token"]


def setup_test_environment(client_id, client_secret):
    token = get_management_token(client_id, client_secret)
    api_url = f"{LOGTO_ENDPOINT}/api"
    headers = {"Authorization": f"Bearer {token}", "Content-Type": "application/json"}

    # 1. Create a Test User (Used for Playwright/Cypress frontend testing)
    print("Creating test user...")
    user_data = {
        "username": "e2e_test_user",
        "primaryEmail": "test@example.com",
        "password": "Password123!",
        "name": "E2E Test Account",
    }
    user_resp = requests.post(f"{api_url}/users", headers=headers, json=user_data)
    if user_resp.status_code == 201:
        print(f"✅ User created. ID: {user_resp.json().get('id')}")
    else:
        print(f"❌ Failed to create user: {user_resp.text}")

    # 2. Create M2M App (Used for testing the backend API logic)
    print("\nCreating internal service M2M app...")
    app_data = {
        "name": "for legal reasons this is game - E2E Tests",
        "type": "MachineToMachine",
        "description": "Test credentials for CI pipeline API tests",
    }
    app_resp = requests.post(f"{api_url}/applications", headers=headers, json=app_data)

    if app_resp.status_code == 201:
        app_info = app_resp.json()
        app_id = app_info.get("id")
        print(f"✅ App created. ID: {app_id}")

        # 3. Fetch the auto-generated secret for the new M2M app
        secret_resp = requests.get(
            f"{api_url}/applications/{app_id}/secrets", headers=headers
        )
        if secret_resp.status_code == 200:
            secrets = secret_resp.json()
            if secrets:
                app_secret = secrets[0].get("value")
                print(f"✅ App Secret: {app_secret}")
                print("\nExport these to your test environment:")
                print(f"export TEST_CLIENT_ID='{app_id}'")
                print(f"export TEST_CLIENT_SECRET='{app_secret}'")
    else:
        print(f"❌ Failed to create app: {app_resp.text}")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python3 seed.py <m-admin-id> <m-admin-secret>")
        sys.exit(1)
    client_id = sys.argv[1].strip()
    client_secret = sys.argv[2].strip()
    print(f"client_id = {client_id}")
    print(f"client_secret = {client_secret}")
    setup_test_environment(client_id, client_secret)

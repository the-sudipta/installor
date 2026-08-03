# Optional Google Sheets write-back

You do **not** need this for public-link imports. This bridge is only for bidirectional synchronization.

1. Open the target Google Sheet and choose **Extensions → Apps Script**.
2. Replace `Code.gs` with this folder's `Code.gs`.
3. Replace `REPLACE_WITH_YOUR_OWN_SECRET` with a long random value.
4. Choose **Deploy → New deployment → Web app**; execute as **Me** and allow access to **Anyone**.
5. Paste the deployment URL and the same secret into Installor Settings, save, then test the connection.

The bridge creates `Catalog` and `Installations` tabs on first use. The secret is the authorization boundary; do not publish or reuse it.

# Installor Google Sheets write-back

You do **not** need this for public-link imports. This bridge is only for bidirectional synchronization.

The easiest and safest route in Installor v0.2.0 is **Settings → Set up Google synchronization**. The in-app wizard generates the secret, stores it in the operating-system credential vault, and shows the complete version-matched script. End users of the standalone application do not need to find this repository file.

1. Open the target Google Sheet and choose **Extensions → Apps Script**.
2. In Installor choose **Copy complete script**, then replace the Apps Script editor's placeholder code.
3. Choose **Deploy → New deployment → Web app**; execute as **Me** and allow access to **Anyone**.
4. Paste the deployment `/exec` URL into the wizard and choose **Save and test connection**.
5. Choose **Push entire local library** to include records created before setup.

Developers and reviewers can inspect the unmodified template in [`Code.gs`](Code.gs). Its `REPLACE_WITH_YOUR_OWN_SECRET` marker is replaced only in memory when the user opens the guided setup; a generated secret is never written into the repository.

The bridge creates `Catalog`, `Installations`, and `Installor Deletions` tabs on first use. The last tab carries removal tombstones between computers. The secret is the authorization boundary; do not publish or reuse it.

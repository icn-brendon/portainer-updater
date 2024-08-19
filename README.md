Portainer Automatic Updater
=====================================

# Overview

This project is designed to automatically update container images using Portainer's webhook support. Note that this software has limited testing and no guarantee of results - use at your own risk!

This is my first projecet any suggestions or feedback would be great! I am new to programming this project will consist of a lot of it is googling, LLM and then understanding and research of said outputs.

# Requirements

*  Postgres database with TLS enabled
*  Machine to compile and run binary (possible releases available)
*  `.env` file created with required information added (see below)

# Goals

The goal of this project is to create a piece of software that can safely upgrade container images to the latest version. Future goals include:

*  Updater checks version & upgrades (mostly done, need full GHCR.io support.)
*  Updater alerts Telegram bot of upgrade, with notification if upgrade succeeds or fails.
*  Detection of major version jumps and notification to Telegram
*  Telegram reactions by user to proceed or deny upgrade process
*  Pull provided website release notes of breaking changes (requires provider to give reliable release note data)
*  Implementation of automatic backup & recovery if upgrade fails
*  WEB UI
*  Adding of container's webhooks via interface or automatically pulled from Portainer

# Install Guide

# Step 1: Enable Webhook Requests in Portainer Business Edition

To enable webhook requests in Portainer Business Edition, follow the instructions here: https://docs.portainer.io/user/docker/services/webhooks

# Step 2: Create a New Postgres Database and Table

Create a new Postgres database for this project using your preferred method (e.g., command line or GUI tool).
Run the following SQL commands to create a new table called `containers`:

<code>
CREATE TABLE containers (
    id SERIAL PRIMARY KEY,
    webhook_url VARCHAR(255) NOT NULL,
    version INT NOT NULL DEFAULT 0,
    namespace VARCHAR(255) NOT NULL,
    repository VARCHAR(255) NOT NULL,
    image_source VARCHAR(50) NOT NULL CHECK (image_source IN ('dockerhub', 'ghcr')),
    arch VARCHAR(20) NOT NULL
);
</code>

# Step 3: Insert Rows Manually for Each Container You Want to Manage

Insert rows manually for each container you want to manage. For example:
<code>
INSERT INTO containers (webhook_url, version, namespace, repository, image_source, arch)
VALUES ('https://example.com/webhooks/12345', 1, 'music-assistant', 'server', 'dockerhub', 'amd64');
</code>

# Step 4: Create a `.env` File with Required Information

Create a `.env` file with the following variables:

`DATABASE_URL`: the URL of your Postgres database (e.g., `postgres://'username':'password'@'localhost:5432'/database_name'`)

`GHCR_TOKEN`: the PAT token from GitHub in base64 format

`TELEGRAM_CHAT_ID`: the chat ID received from Telegram's bot API

`TELEGRAM_BOT_TOKEN`: the token for your Telegram bot

# Contributing
Contributions are welcome! 

# License
This project is licensed under the Apache-2.0 license. See LICENSE.txt for details.

# Disclaimer
This software is untested and should not be used for purpose of anything but lab testing, do not use this in an environements without backups or production or any other use cases in which untested software that could break your enviornment like this would have a negative impact.

# No Warranty
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

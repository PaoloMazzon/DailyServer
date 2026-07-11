#!/usr/bin/env bash
DEB_REVISION=${1:-dev}

if ! [[ -e deploy ]]; then
	echo "Please run this from the project root."
	exit 1
fi

cargo deb --deb-revision "$DEB_REVISION"
cp "$(find . -name "daily-server_*_amd64.deb")" ./daily_server.deb
docker build -f deploy/Dockerfile -t "daily_server:${DEB_REVISION}" .

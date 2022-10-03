#!/bin/bash

echo ################# Build x86 docker image
docker build -t registry.gitlab.com/loers/minicaldav/x86-64:main -f ci/x86-64.dockerfile .
echo ################# Upload x86 image
docker push registry.gitlab.com/loers/minicaldav/x86-64:main

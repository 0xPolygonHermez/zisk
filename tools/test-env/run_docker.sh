#!/bin/bash

set -e

source ./utils.sh

IMAGE_NAME="zisk-test-env"
CONTAINER_NAME="zisk-docker"
OUTPUT_DIR="./output"

mkdir -p "${OUTPUT_DIR}"

# Check if container exists
if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo
    read -p "🚨 Container '${CONTAINER_NAME}' already exists. Connect to it? [Y/n] (choosing 'n' will recreate it): " answer
    answer=${answer:-y}

    case "$answer" in
        [Yy])
            info "🔑 Connecting to existing container..."
            docker start "${CONTAINER_NAME}" >/dev/null
            docker exec -u ziskuser -it "${CONTAINER_NAME}" bash -i -c "./menu.sh"
            exit 0
            ;;
        [Nn])
            info "🚨 Removing existing container..."
            docker stop "${CONTAINER_NAME}" >/dev/null
            docker rm -f "${CONTAINER_NAME}" >/dev/null
            ;;
        *)
            echo "❌ Invalid option: '$answer'. Please enter 'y' or 'n'."
            exit 1
            ;;
    esac
fi

info "🚀 Running docker container ${CONTAINER_NAME}..."
docker run -dit --shm-size=32g --name "${CONTAINER_NAME}" -v "$(realpath "${OUTPUT_DIR}"):/home/ziskuser/output" "${IMAGE_NAME}" bash -l >/dev/null

info "🔑 Accessing the container now..."
docker exec -u ziskuser -it ${CONTAINER_NAME} bash -i -c "sudo chmod 777 /home/ziskuser/output; ./menu.sh"

echo
info "${BOLD}To access the container, run:${RESET} docker exec -u ziskuser -it ${CONTAINER_NAME}  bash -i -c "./menu.sh""
info "${BOLD}To stop the container, run:${RESET} docker stop ${CONTAINER_NAME}"
info "${BOLD}To remove the container, run:${RESET} docker rm -f ${CONTAINER_NAME}"

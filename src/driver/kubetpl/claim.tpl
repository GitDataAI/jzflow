{
  "kind": "PersistentVolumeClaim",
  "apiVersion": "v1",
  "metadata": {
    "name": "{{name}}"
  },
  "spec": {
    "storageClassName": "jz-action-fs",
    "accessModes": [
      "ReadWriteMany"
    ],
    "resources": {
      "requests": {
        "storage": "1Gi"
      }
    }
  }
}

{
  "apiVersion": "apps/v1",
  "kind": "StatefulSet",
  "metadata": {
    "name": "{{{node.name}}}-channel-statefulset",
    "id": "{{{id}}}",
    "labels": {
      "exec-type": "channel"
    }
  },
  "spec": {
    "serviceName": "{{{node.name}}}-channel-headless-service",
    "replicas": 1,
    "selector": {
      "matchLabels": {
        "app": "{{{node.name}}}-channel-pod"
      }
    },
    "template": {
      "metadata": {
        "labels": {
          "app": "{{{node.name}}}-channel-pod"
        }
      },
      "spec": {
        "containers": [
          {
            "name": "channel",
            "image": "jz-action/dp_runner:latest",
            "imagePullPolicy": "IfNotPresent",
            "command": [
              "/dp_runner"
            ],
            "args": [
              "--node-name={{{node.name}}}",
              "--log-level={{{log_level}}}",
              "--mongo-url={{{db.mongo_url}}}",
              "--database={{{run_id}}}"
            ],
            "ports": [
              {
                "containerPort": 80
              }
            ],
            "volumeMounts": [
              {
                "mountPath": "/app/tmp",
                "name": "tmpstore"
              }
            ]
          }
        ],
        "volumes": [
          {
            "name": "tmpstore",
            "persistentVolumeClaim": {
              "claimName": "{{node.name}}-claim"
            }
          }
        ]
      }
    }
  }
}

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
            "image": {{#if node.channel.image}} "{{{node.channel.image}}}"{{else}}"gitdatateam/dp_runner:latest"{{/if}},
            "imagePullPolicy": "IfNotPresent",
            "command": [
              "/dp_runner"
            ],
            "args": [
              "--node-name={{{node.name}}}-channel",
              "--log-level={{{log_level}}}",
              "--mongo-url={{{db_url}}}"
              {{#if (eq node.channel.cache_type "Disk") }},"--tmp-path=/app/tmp"{{/if}}
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
              "claimName": "{{node.name}}-channel-claim"
            }
          }
        ]
      }
    }
  }
}

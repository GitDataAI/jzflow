{
  "apiVersion": "apps/v1",
  "kind": "StatefulSet",
  "metadata": {
    "name": "{{{node.name}}}-statefulset",
    "labels": {
      "exec-type": "compute-unit"
    }
  },
  "spec": {
    "serviceName": "{{{node.name}}}-headless-service",
    "replicas": {{{node.spec.replicas}}},
    "selector": {
      "matchLabels": {
        "app": "{{{node.name}}}-pod"
      }
    },
    "template": {
      "metadata": {
        "labels": {
          "app": "{{{node.name}}}-pod"
        }
      },
      "spec": {
        "containers": [
          {
            "name": "compute-data-unit",
            "image": "gitdatateam/compute_unit_runner:latest",
            "command": [
              "/compute_unit_runner"
            ],
            "args": [
              "--node-name={{{node.name}}}",
              "--log-level={{{log_level}}}",
              "--mongo-url={{{db_url}}}"
               {{#if (eq node.spec.cache_type "Disk") }},"--tmp-path=/app/tmp"{{/if}}
            ],
            "imagePullPolicy": "IfNotPresent",
            "ports": [
              {
                "containerPort": 80
              }
            ],
            "volumeMounts": [
              {
                "mountPath": "/unix_socket",
                "name": "unix-socket"
              },
              {
                "mountPath": "/app/tmp",
                "name": "tmpstore"
              }
            ]
          },
          {
            "name": "compute-user-unit",
            "image": "{{{node.spec.image}}}",
            "command": [
              "{{{node.spec.command}}}"
            ],
            "imagePullPolicy": "IfNotPresent",
            "args": [{{{join_array node.spec.args}}}],
            "volumeMounts": [
              {
                "mountPath": "/unix_socket",
                "name": "unix-socket"
              },
              {
                "mountPath": "/app/tmp",
                "name": "tmpstore"
              }
            ]
          }
        ],
        "volumes": [
          {
            "name": "unix-socket",
            "emptyDir": {}
          },
          {
            "name": "tmpstore",
            "persistentVolumeClaim": {
              "claimName": "{{{node.name}}}-node-claim"
            }
          }
        ]
      }
    }
  }
}

{{/*
Expand the name of the chart.
*/}}
{{- define "data-engine.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "data-engine.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "data-engine.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "data-engine.labels" -}}
helm.sh/chart: {{ include "data-engine.chart" . }}
{{ include "data-engine.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/component: {{ .Values.appName }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "data-engine.selectorLabels" -}}
app.kubernetes.io/name: {{ include "data-engine.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "data-engine.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "data-engine.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
PostgreSQL fullname
*/}}
{{- define "data-engine.postgresql.fullname" -}}
{{- if .Values.customPostgresql.fullnameOverride }}
{{- .Values.customPostgresql.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default "postgresql" .Values.customPostgresql.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
PostgreSQL Secret name
*/}}
{{- define "data-engine.postgresql.secretName" -}}
{{- if .Values.customPostgresql.auth.existingSecret }}
{{- .Values.customPostgresql.auth.existingSecret }}
{{- else }}
{{- printf "%s" (include "data-engine.postgresql.fullname" .) }}
{{- end }}
{{- end }}

{{/*
PostgreSQL host
*/}}
{{- define "data-engine.postgresql.host" -}}
{{- if .Values.customPostgresql.enabled }}
{{- printf "%s-primary" (include "data-engine.postgresql.fullname" .) }}
{{- else }}
{{- .Values.externalDatabase.host }}
{{- end }}
{{- end }}

{{/*
Redis fullname
*/}}
{{- define "data-engine.redis.fullname" -}}
{{- if .Values.customRedis.fullnameOverride }}
{{- .Values.customRedis.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default "redis" .Values.customRedis.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Redis Secret name
*/}}
{{- define "data-engine.redis.secretName" -}}
{{- if .Values.customRedis.auth.existingSecret }}
{{- .Values.customRedis.auth.existingSecret }}
{{- else }}
{{- printf "%s" (include "data-engine.redis.fullname" .) }}
{{- end }}
{{- end }}

{{/*
Return true if cert-manager issued certificate is enabled
*/}}
{{- define "data-engine.ingress.certManagerRequest" -}}
{{- if or (hasKey .Values.ingress.annotations "cert-manager.io/cluster-issuer") (hasKey .Values.ingress.annotations "cert-manager.io/issuer") }}
{{- true -}}
{{- end -}}
{{- end -}}

{{/*
Compile all warnings into a single message.
*/}}
{{- define "data-engine.validateValues" -}}
{{- $messages := list -}}
{{- $messages := append $messages (include "data-engine.validateValues.database" .) -}}
{{- $messages := append $messages (include "data-engine.validateValues.redis" .) -}}
{{- $messages := without $messages "" -}}
{{- $message := join "\n" $messages -}}

{{- if $message -}}
{{-   printf "\nVALUES VALIDATION:\n%s" $message -}}
{{- end -}}
{{- end -}}

{{/*
Validate database configuration
*/}}
{{- define "data-engine.validateValues.database" -}}
{{- if and (not .Values.customPostgresql.enabled) (not .Values.externalDatabase.enabled) -}}
data-engine: database
    You must enable either postgresql or externalDatabase
{{- end -}}
{{- end -}}

{{/*
Validate redis configuration  
*/}}
{{- define "data-engine.validateValues.redis" -}}
{{- if and (not .Values.customRedis.enabled) (not .Values.externalRedis.enabled) -}}
data-engine: redis
    You must enable either redis or externalRedis
{{- end -}}
{{- end -}}

{{/*
Create the name of the service account to use
*/}}
{{- define "DataEngine.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "DataEngine.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

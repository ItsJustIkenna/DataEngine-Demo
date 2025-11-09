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
Return true if cert-manager issued certificate is enabled
*/}}
{{- define "data-engine.ingress.certManagerRequest" -}}
{{- if or (hasKey .Values.ingress.annotations "cert-manager.io/cluster-issuer") (hasKey .Values.ingress.annotations "cert-manager.io/issuer") }}
{{- true -}}
{{- end -}}
{{- end -}}

{{/*
Validate database configuration
*/}}
{{- define "data-engine.validateValues.database" -}}
{{- if not .Values.externalDatabase.enabled -}}
data-engine: database
    You must enable externalDatabase
{{- end -}}
{{- end -}}

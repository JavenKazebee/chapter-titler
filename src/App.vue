<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { marked } from "marked";
import { invoke } from "@tauri-apps/api/core";
import { InputMask, InputText } from "primevue";
import { load } from "@tauri-apps/plugin-store";
import { listen } from "@tauri-apps/api/event";
import { useToast } from 'primevue/usetoast';
import { check, type Update } from "@tauri-apps/plugin-updater";
import { getVersion } from '@tauri-apps/api/app';

// -----------------------------------------------------------------------------
// Const (state, types, store)
// -----------------------------------------------------------------------------

// Toast / rate limit
const toast = useToast();
const timeRemaining = ref("");
const rateLimitIntervalId = ref<ReturnType<typeof setInterval> | null>(null);

// Main page
const videoId = ref("");
const chapterTitles = ref("");
const offset = ref(false);
const offsetTime = ref("");

// Dialog
const vimeoSettingsDialog = ref(false);
const vimeoAccessToken = ref("");

// Upload
const isUploading = ref(false);
const progress = ref(0);
const statusMessage = ref("");
const uploadResult = ref<UploadResult | null>(null);

// Tauri Store
let store: any = null;

interface ProgressPayload {
  current: number;
  total: number;
  title: string;
}

interface UploadResult {
  successful: number;
  total: number;
  error?: {
    type: string;
    data?: {
      message?: string;
      line_number?: number;
      raw_line?: string;
      reset_time?: string;
    };
  };
}

function formatUploadError(error: { type: string; data?: Record<string, unknown> }): {
  title: string;
  detail: string;
} {
  const t = error.type;
  const d = error.data ?? {};
  const msg = d.message as string | undefined;
  switch (t) {
    case "Parse":
      return {
        title: "Parse Error",
        detail: `Line ${d.line_number} - ${msg} ("${d.raw_line}")`,
      };
    case "Auth":
      return { title: "Authentication Error", detail: msg ?? "Authentication failed" };
    case "Vimeo":
      return { title: "Vimeo Error", detail: msg ?? "Vimeo API error" };
    case "RateLimit":
      return { title: "Rate Limit Error", detail: msg ?? "You've been rate limited." };
    case "Offset":
      return { title: "Offset Error", detail: msg ?? "Invalid offset" };
    default:
      return { title: "Error", detail: (msg as string) ?? "An unknown error has occurred." };
  }
}

// Updater
const currentVersion = ref("");
const pendingUpdate = shallowRef<Update | null>(null);
const updateAvailable = ref(false);
const isUpdating = ref(false);
const releaseNotesDialog = ref(false);

// -----------------------------------------------------------------------------
// Lifecycle
// -----------------------------------------------------------------------------

onMounted(async () => {
  store = await load('data.json');
  vimeoAccessToken.value = await store.get('access_token');
  checkForUpdates();
  currentVersion.value = await getVersion();
});

onUnmounted(() => {
  if (rateLimitIntervalId.value !== null) {
    clearInterval(rateLimitIntervalId.value);
  }
});

// -----------------------------------------------------------------------------
// Other functions
// -----------------------------------------------------------------------------

function formatTimeRemaining(secondsLeft: number): string {
  const hours = Math.floor(secondsLeft / 3600);
  const minutes = Math.floor((secondsLeft % 3600) / 60);
  const seconds = Math.floor(secondsLeft % 60);
  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  parts.push(`${seconds}s`);
  return parts.join(" ");
}

function startRateLimitCountdown(resetTimeStr: string) {
  // Vimeo X-RateLimit-Reset is Unix seconds; allow ISO strings too
  const resetMs = /^\d+$/.test(resetTimeStr)
    ? parseInt(resetTimeStr, 10) * 1000
    : new Date(resetTimeStr).getTime();
  if (Number.isNaN(resetMs)) {
    timeRemaining.value = resetTimeStr;
    return;
  }
  function tick() {
    const left = (resetMs - Date.now()) / 1000;
    timeRemaining.value = formatTimeRemaining(left);
    if (left <= 0 && rateLimitIntervalId.value !== null) {
      clearInterval(rateLimitIntervalId.value);
      rateLimitIntervalId.value = null;
      timeRemaining.value = "";
    }
  }
  tick();
  rateLimitIntervalId.value = setInterval(tick, 1000);
}

async function saveAuthentication() {
  if (!store) {
    toast.add({ severity: 'error', summary: "Error", detail: 'Failed to save authentication data.', life: 5000, group: 'bc' });
    return;
  }
  vimeoSettingsDialog.value = false;
  await store.set('access_token', vimeoAccessToken.value);
}

async function upload(startIndex: number) {
  isUploading.value = true;
  uploadResult.value = null;
  const listener = await listen<ProgressPayload>("upload-progress", (event) => {
    const { current, total, title } = event.payload;
    progress.value = Math.round((current / total) * 100);
    statusMessage.value = `Uploading: ${title}`;
  });

  try {
    uploadResult.value = await invoke<UploadResult>('upload_chapter_titles', {
      videoId: videoId.value,
      text: chapterTitles.value,
      offset: offset.value ? offsetTime.value : "00:00",
      startIndex
    });

    console.log(uploadResult.value);
    console.log(`Is Successful: ${uploadCardSuccess.value}`);
    console.log(`Is Rate Limit: ${uploadResultIsRateLimit.value}`);
    console.log(`Failed Count: ${uploadCardFailedCount.value}`);
    if (uploadResult.value.error?.type === 'RateLimit' && uploadResult.value.error?.data?.reset_time) {
      startRateLimitCountdown(uploadResult.value.error.data.reset_time);
    }
  } catch (err: any) {
    const error =
      err?.type != null && err?.data != null ? err : { type: 'Error', data: { message: err?.message ?? 'An unknown error has occurred.' } };
    uploadResult.value = { successful: 0, total: 0, error };
  } finally {
    listener();
    isUploading.value = false;
    progress.value = 0;
    statusMessage.value = "";
  }
}

async function defaultOffset() {
  if (offset.value) {
    try {
      offsetTime.value = await invoke('get_default_offset', {
        text: chapterTitles.value,
      });
    } catch(err: any) {
      offsetTime.value = "00:00:00";
    }
    
  }
}

async function checkForUpdates() {
  try {
    const update = await check();
    if (update) {
      pendingUpdate.value = update;
      updateAvailable.value = true;
    }
  } catch (err: any) {
    console.error(err);
  }
}

async function checkForUpdatesFromSettings() {
  vimeoSettingsDialog.value = false;
  try {
    const update = await check();
    if (update) {
      pendingUpdate.value = update;
      updateAvailable.value = true;
      toast.add({ severity: "success", summary: "Update available", detail: `Version ${update.version} is available.`, life: 3000, group: "tr" });
    } else {
      toast.add({ severity: "info", summary: "Up to date", detail: "You're running the latest version.", life: 3000, group: "tr" });
    }
  } catch (err: any) {
    toast.add({ severity: "error", summary: "Update check failed", detail: err?.message ?? "Could not check for updates.", group: "tr" });
  }
}

async function downloadUpdate() {
  const update = pendingUpdate.value;
  if (!update) return;
  isUpdating.value = true;
  try {
    await update.downloadAndInstall();
    toast.add({ severity: "success", summary: "Update installed", detail: "Restart the app to complete the update.", group: "tr" });
  } catch (err: any) {
    toast.add({ severity: "error", summary: "Update failed", detail: err?.message ?? "Download failed.", group: "tr" });
  } finally {
    isUpdating.value = false;
  }
}

function showReleaseNotes() {
  releaseNotesDialog.value = true;
}

const releaseNotesHtml = computed(() => {
  const body = pendingUpdate.value?.body ?? "No release notes available.";
  return marked.parse(body, { async: false }) as string;
});

const showUploadCard = computed(() => isUploading.value || uploadResult.value !== null);
const uploadCardSuccess = computed(() => {
  const r = uploadResult.value;
  return r !== null && !r.error;
});
const uploadCardFailedCount = computed(() => {
  const r = uploadResult.value;
  if (!r?.error) return 0;
  return r.total - r.successful;
});
const uploadResultIsRateLimit = computed(() => uploadResult.value?.error?.type === 'RateLimit');
const uploadErrorFormatted = computed(() => {
  const err = uploadResult.value?.error;
  return err ? formatUploadError(err) : null;
});
const uploadResultIsPartialSuccess = computed(() => {
  const r = uploadResult.value;
  return r !== null && r.successful >= 1 && r.successful < r.total;
});
</script>

<template>
  <main>
    <Toast position="bottom-center" group="bc">
      <template #message="{ message }">
        <div v-if="message.summary === 'Rate Limit Error'" class="flex flex-col gap-1">
          <span class="font-semibold">{{ message.summary }}</span>
          <span>{{ message.detail }} Try again in <strong>{{ timeRemaining }}</strong></span>
        </div>
        <template v-else>
          <span class="font-semibold">{{ message.summary }}</span>
          <span>{{ message.detail }}</span>
        </template>
      </template>
    </Toast>

    <Toast position="top-right" group="tr"/>

    <div
      v-if="updateAvailable"
      class="w-full py-2 px-4 flex items-center justify-between text-sm font-semibold shadow bg-[var(--p-primary-500)] text-[var(--p-primary-contrast)]"
    >
      <span>Update available: <span class="font-bold">{{ pendingUpdate?.version ?? 'unknown' }}</span> (current version: {{ currentVersion }})</span>
      <div class="flex items-center gap-2">
        <Button label="What's new?" size="small" severity="contrast" @click="showReleaseNotes" />
        <Button icon="pi pi-download" severity="contrast" size="small" @click="downloadUpdate" :loading="isUpdating" />
      </div>
    </div>

    <div class="flex flex-col items-center p-4 gap-4">

      <div class="flex w-full items-center justify-between">
        <div class="w-12"></div>
        <div class="text-4xl font-bold">Chapter Titler</div>
        <Button class="" icon="pi pi-cog" @click="vimeoSettingsDialog=true"/>
      </div>

      <InputText v-model="videoId" placeholder="Video ID"/>

      <Textarea v-model="chapterTitles" style="resize: none" placeholder="Chapter titles" rows="15" cols="50"/>

      <div class="flex gap-4">
        <div class="flex gap-2 items-center">
          <Checkbox v-model="offset" inputId="offset" binary @change="defaultOffset"/>
          <label for="offset">Offset</label>
        </div>

        <InputMask v-model="offsetTime" mask="99:99:99" placeholder="HH:MM:SS" :disabled="!offset"/>
      </div>

      <div class="flex flex-col items-center gap-2 w-full max-w-md">
        <Button label="Upload" @click="upload(0)" :disabled="isUploading || !videoId.trim() || !chapterTitles.trim()"/>
        <div
          v-if="showUploadCard"
          :class="[
            'w-full transition-colors rounded-lg overflow-hidden'
          ]"
        >
        <Card
          class="w-full"
          :class="uploadResult
            ? (uploadCardSuccess ? 'border-3 !bg-green-500/5 border-green-500/50' : 'border-3 !bg-red-500/5 border-red-500/50')
            : ''"
        >
          <template #content>
            <template v-if="isUploading">
              <div class="flex flex-col gap-1 upload-progress">
                <ProgressBar :value="progress" />
                <span class="text-sm text-surface-600 truncate text-center">{{ statusMessage }}</span>
              </div>
            </template>
            <template v-else-if="uploadResult">
              <div class="flex items-center justify-between gap-4">
                <div class="flex flex-col gap-1">
                  <span class="font-medium">
                    {{ uploadResult.successful }}/{{ uploadResult.total }} uploaded
                  </span>
                  <template v-if="uploadErrorFormatted">
                    <span class="text-sm font-semibold">{{ uploadErrorFormatted.title }}</span>
                    <span class="text-sm">{{ uploadErrorFormatted.detail }}</span>
                    <div
                      v-if="uploadResultIsRateLimit && timeRemaining"
                      class="text-sm mt-1"
                    >
                      Try again in <strong>{{ timeRemaining }}</strong>
                    </div>
                  </template>
                </div>
                <Button
                  v-if="uploadResultIsPartialSuccess"
                  label="Finish Upload"
                  severity="secondary"
                  @click="upload(uploadResult.successful)"
                />
              </div>
            </template>
          </template>
        </Card>
        </div>
      </div>
    </div>


    <Dialog v-model:visible="vimeoSettingsDialog" modal header="Vimeo Authentication">
      <div class="flex flex-col items-center gap-4">
        <div class="flex gap-2 w-full">
          <InputText v-model="vimeoAccessToken" placeholder="Access Token" class="flex-1"/>
          <Button label="Save" @click="saveAuthentication"/>
        </div>
        <Button class="mt-8"label="Check for Updates" severity="secondary" @click="checkForUpdatesFromSettings"/>
        <span class="text-sm text-slate-500 font-bold">Version {{ currentVersion }}</span>
      </div>
    </Dialog>

    <Dialog v-model:visible="releaseNotesDialog" modal header="Release Notes" class="max-w-lg">
      <div class="prose prose-sm max-w-none prose-invert" v-html="releaseNotesHtml"></div>
    </Dialog>
  </main>
</template>

<style scoped>
/* Snappy progress bar so it keeps up with fast upload updates */
.upload-progress :deep(.p-progressbar-value) {
  transition-duration: 0.3s;
}
</style>
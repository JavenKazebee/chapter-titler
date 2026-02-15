<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { InputMask, InputText } from "primevue";
import { load } from "@tauri-apps/plugin-store";
import { listen } from "@tauri-apps/api/event";
import { useToast } from 'primevue/usetoast';

const toast = useToast();

// Main page
const videoId = ref("");
const chapterTitles = ref("");
const offset = ref(false);
const offsetTime = ref("");

// Dialog
// TODO remove clientSecret and clientID from app
const vimeoSettingsDialog = ref(false);
const vimeoClientID = ref("");
const vimeoClientSecret = ref("");
const vimeoAccessToken = ref("");

// Upload
const isUploading = ref(false);
const progress = ref(0);
const statusMessage = ref("");

// Tauri Store
let store: any = null;

interface ProgressPayload {
  current: number;
  total: number;
  title: string;
}

onMounted(async () => {
  store = await load('data.json');

  vimeoAccessToken.value = await store.get('access_token');
})

async function saveAuthentication() {
  if(!store.value) {
    toast.add({ severity: 'error', summary: "Error", detail: 'Failed to save authentication data.', life: 5000, group: 'bc' });
    return;
  }

  vimeoSettingsDialog.value = false;
  await store.set('access_token', vimeoAccessToken.value);
}

async function upload() {
  // Reset state
  isUploading.value = true;

  // Listen for progress from Rust
  const listener = await listen<ProgressPayload>("upload-progress", (event) => {
    const { current, total, title } = event.payload;
    progress.value = Math.round((current / total) * 100);
    statusMessage.value = `Uploading: ${title}`;
    
  })

  try {
    await invoke('upload_chapter_titles', { 
      videoId: videoId.value,
      text: chapterTitles.value,
      offset: offset.value ? offsetTime.value : "00:00",
    });
  } catch (err: any) {
    const { type, data } = err;

    // TODO improve error handling
    if (type === 'Parse') {
      toast.add({ severity: 'error', summary: 'Parsing Error', detail: `Typo on line ${data.line_number}: ${data.message}\n"${data.raw_line}"`, group: 'bc' });
    } else if (type === 'Auth') {
      toast.add({ severity: 'error', summary: 'Authentication Error', detail: `Authentication Error: ${data.message}`, group: 'bc' });
    } else if (type === 'Vimeo') {
      toast.add({ severity: 'error', summary: 'Vimeo Error', detail: `Vimeo Error: ${data.message}`, group: 'bc' });
    } else {
      toast.add({ severity: 'error', summary: 'Unknown Error', detail: 'An uknown error has occured.', group: 'bc' });
    }
  } finally {
    listener(); // stop listening
    isUploading.value = false;
    progress.value = 0;
    statusMessage.value = "";
  }
}

async function defaultOffset() {
  if(offset.value) {
    
  }
}
</script>

<template>
  <main>
    <Toast position="bottom-center" group="bc"/>
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
        <Button label="Upload" @click="upload" :disabled="isUploading"/>
        <template v-if="isUploading">
          <div class="w-full flex flex-col gap-1">
            <ProgressBar :value="progress" />
            <span class="text-sm text-surface-600 truncate text-center">{{ statusMessage }}</span>
          </div>
        </template>
      </div>
    </div>


    <Dialog v-model:visible="vimeoSettingsDialog" modal header="Vimeo Authentication">
      <div class="flex flex-col items-center gap-4">
        <InputText v-model="vimeoClientID" placeholder="Client ID"/>
        <InputText v-model="vimeoClientSecret" placeholder="Client Secret"/>
        <InputText v-model="vimeoAccessToken" placeholder="Access Token"/>
        <Button label="Save" @click="saveAuthentication"/>
      </div>
    </Dialog>
  </main>
</template>
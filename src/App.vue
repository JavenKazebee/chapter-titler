<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { InputMask, InputNumber, InputText } from "primevue";

const greetMsg = ref("");
const name = ref("");

// Main page
const videoUrl = ref("");
const chapterTitles = ref("");
const offset = ref(false);
const offsetTime = ref("");

// Dialog
const vimeoSettingsDialog = ref(false);
const vimeoClientID = ref("");
const vimeoClientSecret = ref("");
const vimeoAccessToken = ref("");

async function greet() {
  greetMsg.value = await invoke("greet", { name: name.value });
}

async function saveAuthentication() {
  vimeoSettingsDialog.value = false;
}

async function upload() {

}

async function defaultOffset() {
  if(offset.value) {
    
  }
}
</script>

<template>
  <main>
    <div class="flex flex-col items-center p-4 gap-4">

      <div class="flex w-full items-center justify-between">
        <div class="w-12"></div>
        <div class="text-4xl font-bold">Chapter Titler</div>
        <Button class="" icon="pi pi-cog" @click="vimeoSettingsDialog=true"/>
      </div>

      <InputText v-model="videoUrl" placeholder="Video URL"/>

      <Textarea v-model="chapterTitles" style="resize: none" placeholder="Chapter titles" rows="15" cols="50"/>

      <div class="flex gap-4">
        <div class="flex gap-2 items-center">
          <Checkbox v-model="offset" inputId="offset" binary @change="defaultOffset"/>
          <label for="offset">Offset</label>
        </div>

        <InputMask v-model="offsetTime" mask="99:99:99" placeholder="HH:MM:SS" :disabled="!offset"/>
      </div>

      <div class="">
        <Button label="Upload" @click="upload"/>
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
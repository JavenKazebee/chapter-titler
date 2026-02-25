import { createApp } from "vue";
import App from "./App.vue";

import PrimeVue from 'primevue/config';
import Aura from '@primeuix/themes/aura';
import 'primeicons/primeicons.css';
import ToastService from 'primevue/toastservice';

import Button from "primevue/button";
import InputText from 'primevue/inputtext';
import Textarea from 'primevue/textarea';
import Checkbox from 'primevue/checkbox';
import InputMask from 'primevue/inputmask';
import Dialog from 'primevue/dialog';
import ProgressBar from "primevue/progressbar";
import Toast from 'primevue/toast';
import Card from 'primevue/card';

const app = createApp(App);
app.use(PrimeVue, {
    theme: {
        preset: Aura,
        options: {
            darkModeSelector: '.dark',
        }
    }
});
app.use(ToastService);
app.component('Button', Button);
app.component('Toast', Toast);
app.component('InputText', InputText);
app.component('Textarea', Textarea);
app.component('Checkbox', Checkbox);
app.component('InputMask', InputMask);
app.component('Dialog', Dialog);
app.component('ProgressBar', ProgressBar);
app.component('Card', Card);

app.mount("#app");

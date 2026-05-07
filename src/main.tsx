import React from 'react'
import ReactDOM from 'react-dom/client'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import './i18n'
import './index.css'
import App from './App'
import StatusBar from './StatusBar'
import UpdateToast from './UpdateToast'

const label = getCurrentWebviewWindow().label
const isPill = label === 'status-bar'
const isToast = label === 'update-toast'

if (isPill || isToast) {
  document.documentElement.style.background = 'transparent'
  document.body.style.background = 'transparent'
}

function rootView() {
  if (isPill) return <StatusBar />
  if (isToast) return <UpdateToast />
  return <App />
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>{rootView()}</React.StrictMode>
)

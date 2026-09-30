package com.picsartclone.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.picsartclone.engine.EditorEngine

// Vulkan canvas: VkImage / HardwareBuffer imported from the Rust render plan.
// Touch → EditorEngine → renderPreview() → Vulkan command buffer → Surface.
// Adreno/Mali/Xclipse handled via Vulkan portability; thermal listener sheds resolution.

// Full-screen canvas + bottom toolbar. Gestures update Rust transform uniforms;
// sliders send fused color JSON with a gesture id (100 ticks → 1 undo entry).
@Composable
fun EditorScreen(engine: EditorEngine = remember { EditorEngine() }) {
    var brightness by remember { mutableStateOf(0f) }
    var gesture by remember { mutableStateOf<String?>(null) }
    var perfText by remember { mutableStateOf("FPS: --") }

    Column(Modifier.fillMaxSize()) {
        Box(Modifier.weight(1f).fillMaxWidth()) {
            AndroidView(factory = { ctx ->
                // Real impl: VulkanSurfaceView encoding plan JSON from
                // engine.renderPreview(w, h, interacting) each Choreographer frame.
                android.view.SurfaceView(ctx)
            }, modifier = Modifier.fillMaxSize())
            Text(perfText, style = MaterialTheme.typography.labelSmall, modifier = Modifier.padding(8.dp))
        }
        // Background removal entry point (AI queue only, never UI thread):
        // engine queues segmentation; result mask → setMask → cached + undoable.
        Slider(value = brightness, onValueChange = {
            brightness = it
            if (gesture == null) gesture = java.util.UUID.randomUUID().toString()
            engine.setFilter(1L, colorJson(it), gesture)
        }, onValueChangeFinished = { engine.endGesture(); gesture = null }, valueRange = -1f..1f)
        Row(Modifier.padding(8.dp), horizontalArrangement = Arrangement.SpaceEvenly) {
            Button(onClick = { engine.undo() }) { Text("Undo") }
            Button(onClick = { engine.redo() }) { Text("Redo") }
            Button(onClick = { /* enqueue export on IO dispatcher */ }) { Text("Export") }
        }
    }
}

private fun colorJson(b: Float) =
    """{"brightness":$b,"contrast":1.0,"saturation":1.0,"exposure":0.0,"temperature":0.0,"tint":0.0,"hue_shift":0.0,"sharpen":0.0,"blur_radius":0.0,"vignette":0.0,"grain":0.0}"""

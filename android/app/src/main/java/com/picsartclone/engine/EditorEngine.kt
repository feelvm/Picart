package com.picsartclone.engine

// JNI bridge → Rust C ABI (libeditor_core.so). All calls return JSON strings;
// pixel buffers never cross JNI on the hot path — only render-plan JSON.
class EditorEngine {
    external fun createDocument(width: Int, height: Int): String
    external fun addImage(specJson: String): String
    external fun setFilter(layerId: Long, colorJson: String, gesture: String?): String
    external fun setTransform(layerId: Long, transformJson: String, gesture: String?): String
    external fun setOpacity(layerId: Long, opacity: Float, gesture: String?): String
    external fun moveLayer(layerId: Long, toIndex: Int): String
    external fun removeLayer(layerId: Long): String
    external fun undo(): String
    external fun redo(): String
    external fun endGesture()
    external fun renderPreview(viewportW: Int, viewportH: Int, interacting: Boolean): String
    external fun saveDocument(): String
    external fun loadDocument(json: String): String

    companion object {
        init { System.loadLibrary("editor_core") }
    }
}

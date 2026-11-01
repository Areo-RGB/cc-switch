import {
  invokeWithDevFallback,
  DEV_VIDEO_CLIPS,
  DEV_VIDEO_LIBRARY,
} from "./devMocks";

export interface VideoEntry {
  id: string;
  title: string;
  duration: string;
  clipCount: number;
  thumbnail?: string;
  path?: string;
}

export interface VideoClip {
  id: string;
  title: string;
  start: string;
  duration: string;
}

export interface VideoLibrary {
  folder: string;
  entries: VideoEntry[];
}

export interface AnalyzeOptions {
  model: string;
  thinking: "low" | "medium" | "high";
  mediaResolution: "low" | "medium" | "high";
  fps: number;
  agentic: boolean;
}

export interface VideoJob {
  id: string;
  status: "queued" | "running" | "complete";
}

export interface VideoAnalysis {
  title: string;
  chapters: Array<{ title: string; start: string; end: string }>;
  log: string[];
}

export const videoApi = {
  library(folder?: string) {
    return invokeWithDevFallback<VideoLibrary>(
      "video_library",
      { folder: folder ?? null },
      () => ({
        ...DEV_VIDEO_LIBRARY,
        folder: folder ?? DEV_VIDEO_LIBRARY.folder,
      }),
    );
  },
  clips(videoId: string) {
    return invokeWithDevFallback<VideoClip[]>(
      "video_clips",
      { videoId },
      () => DEV_VIDEO_CLIPS[videoId] ?? [],
    );
  },
  openFolder(path: string) {
    return invokeWithDevFallback<void>(
      "video_open_folder",
      { path },
      () => undefined,
    );
  },
  analyze(url: string, chaptersJson: string, options: AnalyzeOptions) {
    return invokeWithDevFallback<VideoAnalysis>(
      "video_analyze",
      { url, chaptersJson, options },
      () => ({
        title: "AI Studio analysis",
        chapters: [
          { title: "Introduction", start: "00:00", end: "01:12" },
          { title: "Main topic", start: "01:12", end: "03:40" },
        ],
        log: [
          "Queued AI Studio analysis",
          "Uploading media",
          "Extracting chapters",
        ],
      }),
    );
  },
  download(url: string, chaptersJson: string, options: AnalyzeOptions) {
    return invokeWithDevFallback<VideoJob>(
      "video_download",
      { url, chaptersJson, options },
      () => ({ id: "dev-video-job", status: "queued" }),
    );
  },
  stop(jobId: string) {
    return invokeWithDevFallback<void>(
      "video_stop",
      { jobId },
      () => undefined,
    );
  },
};

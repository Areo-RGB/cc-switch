import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  ArrowLeft,
  FolderOpen,
  Loader2,
  Play,
  PlaySquare,
  Sparkles,
  Upload,
} from "lucide-react";
import {
  videoApi,
  type AnalyzeOptions,
  type VideoEntry,
} from "@/lib/api/video";
import { AppPageHeader } from "@/components/shell/AppPageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { PageTabs } from "@/components/ui/page-tabs";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { toast } from "@/lib/toast";
import { cn } from "@/lib/utils";

type VideoTab = "download" | "library";

const defaultOptions: AnalyzeOptions = {
  model: "Gemini 2.5 Pro",
  thinking: "medium",
  mediaResolution: "high",
  fps: 1,
  agentic: true,
};

export function VideoPage() {
  const [tab, setTab] = useState<VideoTab>("library");
  const [url, setUrl] = useState("");
  const [chapters, setChapters] = useState("[]");
  const [options, setOptions] = useState(defaultOptions);
  const [log, setLog] = useState<string[]>([
    "Ready for an AI Studio analysis.",
  ]);
  const [player, setPlayer] = useState<VideoEntry | null>(null);
  const [clipId, setClipId] = useState<string | null>(null);
  const library = useQuery({
    queryKey: ["video", "library"],
    queryFn: () => videoApi.library(),
  });
  const clips = useQuery({
    queryKey: ["video", "clips", player?.id],
    queryFn: () => videoApi.clips(player?.id ?? ""),
    enabled: Boolean(player),
  });
  const analyze = async () => {
    if (!url.trim()) {
      toast.error("Enter a video URL first");
      return;
    }
    setLog(["Queued AI Studio analysis", "Uploading media…"]);
    try {
      const result = await videoApi.analyze(url, chapters, options);
      setLog(result.log);
      setChapters(JSON.stringify(result.chapters, null, 2));
      toast.success("Analysis complete");
    } catch (error) {
      toast.error(String(error));
      setLog((current) => [...current, "Analysis failed"]);
    }
  };
  const download = async () => {
    try {
      await videoApi.download(url, chapters, options);
      setLog((current) => [...current, "Download queued"]);
      toast.success("Download queued");
    } catch (error) {
      toast.error(String(error));
    }
  };

  if (player)
    return (
      <PlayerView
        video={player}
        clips={clips.data ?? []}
        clipId={clipId}
        onClip={setClipId}
        onBack={() => {
          setPlayer(null);
          setClipId(null);
        }}
      />
    );
  return (
    <>
      <AppPageHeader
        icon={<PlaySquare className="h-5 w-5" strokeWidth={1.5} />}
        title="Video"
      />
      <div className="shrink-0 px-6 pt-2">
        <PageTabs<VideoTab>
          aria-label="Video sections"
          value={tab}
          onValueChange={setTab}
          items={[
            { value: "download", label: "Download" },
            { value: "library", label: "Library" },
          ]}
        />
      </div>
      {tab === "download" ? (
        <DownloadView
          url={url}
          setUrl={setUrl}
          chapters={chapters}
          setChapters={setChapters}
          options={options}
          setOptions={setOptions}
          log={log}
          onAnalyze={() => void analyze()}
          onDownload={() => void download()}
        />
      ) : (
        <LibraryView
          library={library.data?.entries ?? []}
          folder={library.data?.folder ?? ""}
          onOpenFolder={() =>
            void videoApi.openFolder(library.data?.folder ?? "")
          }
          onOpenVideo={setPlayer}
        />
      )}
    </>
  );
}

function DownloadView({
  url,
  setUrl,
  chapters,
  setChapters,
  options,
  setOptions,
  log,
  onAnalyze,
  onDownload,
}: {
  url: string;
  setUrl: (value: string) => void;
  chapters: string;
  setChapters: (value: string) => void;
  options: AnalyzeOptions;
  setOptions: (value: AnalyzeOptions) => void;
  log: string[];
  onAnalyze: () => void;
  onDownload: () => void;
}) {
  return (
    <main
      id="main-content"
      className="min-h-0 flex-1 overflow-y-auto px-6 pb-12 pt-4"
    >
      <div className="mx-auto grid max-w-[1080px] gap-4 lg:grid-cols-[1.2fr_0.8fr]">
        <Card>
          <CardHeader className="p-5 pb-3">
            <CardTitle className="text-body font-semibold">
              Download a video
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 p-5 pt-1">
            <div>
              <label className="mb-1.5 block text-caption text-fg-2">
                Video URL
              </label>
              <Input
                value={url}
                onChange={(event) => setUrl(event.target.value)}
                placeholder="https://youtube.com/watch?v=…"
              />
            </div>
            <div>
              <label className="mb-1.5 block text-caption text-fg-2">
                Chapters JSON
              </label>
              <Textarea
                value={chapters}
                onChange={(event) => setChapters(event.target.value)}
                className="min-h-[150px] font-mono text-caption"
                spellCheck={false}
              />
            </div>
            <div className="flex flex-wrap gap-2">
              <Button variant="neutral" onClick={onAnalyze}>
                <Sparkles className="h-4 w-4" />
                Analyze in AI Studio
              </Button>
              <Button variant="solid" onClick={onDownload}>
                <Upload className="h-4 w-4" />
                Download &amp; cut
              </Button>
            </div>
          </CardContent>
        </Card>
        <div className="space-y-4">
          <Card>
            <CardHeader className="p-5 pb-3">
              <CardTitle className="text-body font-semibold">
                Analyze options
              </CardTitle>
            </CardHeader>
            <CardContent className="grid gap-3 p-5 pt-1 sm:grid-cols-2">
              <FieldSelect
                label="Model"
                value={options.model}
                options={["Gemini 2.5 Pro", "Gemini 2.5 Flash"]}
                onChange={(model) => setOptions({ ...options, model })}
              />
              <FieldSelect
                label="Thinking"
                value={options.thinking}
                options={["low", "medium", "high"]}
                onChange={(thinking) =>
                  setOptions({
                    ...options,
                    thinking: thinking as AnalyzeOptions["thinking"],
                  })
                }
              />
              <FieldSelect
                label="Media resolution"
                value={options.mediaResolution}
                options={["low", "medium", "high"]}
                onChange={(mediaResolution) =>
                  setOptions({
                    ...options,
                    mediaResolution:
                      mediaResolution as AnalyzeOptions["mediaResolution"],
                  })
                }
              />
              <label className="flex flex-col gap-1 text-caption text-fg-2">
                FPS
                <input
                  type="number"
                  min={0.5}
                  max={5}
                  step={0.5}
                  value={options.fps}
                  onChange={(event) =>
                    setOptions({ ...options, fps: Number(event.target.value) })
                  }
                  className="h-8 rounded-control border border-border-strong bg-surface px-2 text-body text-fg-1"
                />
              </label>
              <label className="col-span-full flex items-center gap-2 text-caption text-fg-2">
                <Switch
                  size="sm"
                  checked={options.agentic}
                  onCheckedChange={(agentic) =>
                    setOptions({ ...options, agentic })
                  }
                />
                Agentic video understanding
              </label>
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="p-5 pb-3">
              <CardTitle className="text-body font-semibold">
                Live log
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-1 rounded-b-lg bg-subtle p-4 font-mono text-caption text-fg-2">
              {log.map((line, index) => (
                <div key={`${line}-${index}`}>
                  <span className="me-2 text-fg-3">
                    {String(index + 1).padStart(2, "0")}
                  </span>
                  {line}
                </div>
              ))}
              <div className="mt-2 flex items-center gap-2 text-fg-3">
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                waiting for a job
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </main>
  );
}

function FieldSelect({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: string[];
  onChange: (value: string) => void;
}) {
  return (
    <label className="flex flex-col gap-1 text-caption text-fg-2">
      {label}
      <select
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-8 rounded-control border border-border-strong bg-surface px-2 text-body text-fg-1"
      >
        {options.map((option) => (
          <option key={option}>{option}</option>
        ))}
      </select>
    </label>
  );
}

function LibraryView({
  library,
  folder,
  onOpenFolder,
  onOpenVideo,
}: {
  library: VideoEntry[];
  folder: string;
  onOpenFolder: () => void;
  onOpenVideo: (video: VideoEntry) => void;
}) {
  return (
    <main
      id="main-content"
      className="min-h-0 flex-1 overflow-y-auto px-6 pb-12 pt-4"
    >
      <div className="mx-auto max-w-[1080px] space-y-4">
        <div className="flex items-center gap-2">
          <FolderOpen className="h-4 w-4 text-fg-3" />
          <Input
            value={folder}
            readOnly
            className="max-w-[520px] font-mono text-caption"
          />
          <Button variant="neutral" onClick={onOpenFolder}>
            Open
          </Button>
          <Badge
            variant="outline"
            className="ms-auto border-0 bg-subtle text-caption"
          >
            {library.length} videos
          </Badge>
        </div>
        <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {library.map((video) => (
            <button
              key={video.id}
              type="button"
              onClick={() => onOpenVideo(video)}
              className="group overflow-hidden rounded-panel border border-border bg-surface text-start shadow-sm transition-colors hover:border-border-strong hover:bg-subtle"
            >
              <div className="relative aspect-video overflow-hidden bg-subtle">
                {video.thumbnail && (
                  <img
                    src={video.thumbnail}
                    alt=""
                    className="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
                  />
                )}
                <span className="absolute inset-0 flex items-center justify-center bg-black/10 text-white opacity-0 transition-opacity group-hover:opacity-100">
                  <span className="flex h-10 w-10 items-center justify-center rounded-full bg-black/55">
                    <Play className="ms-0.5 h-4 w-4 fill-current" />
                  </span>
                </span>
                <span className="absolute bottom-2 end-2 rounded bg-black/70 px-1.5 py-0.5 text-[11px] text-white">
                  {video.duration}
                </span>
              </div>
              <div className="p-3">
                <div className="truncate text-body font-medium text-fg-1">
                  {video.title}
                </div>
                <div className="mt-1 text-caption text-fg-3">
                  {video.duration} · {video.clipCount} clips
                </div>
              </div>
            </button>
          ))}
        </div>
      </div>
    </main>
  );
}

function PlayerView({
  video,
  clips,
  clipId,
  onClip,
  onBack,
}: {
  video: VideoEntry;
  clips: Array<{ id: string; title: string; start: string; duration: string }>;
  clipId: string | null;
  onClip: (id: string) => void;
  onBack: () => void;
}) {
  return (
    <>
      <AppPageHeader
        leading={
          <Button
            variant="quiet"
            size="icon-compact"
            aria-label="Back to library"
            onClick={onBack}
          >
            <ArrowLeft className="h-4 w-4" />
          </Button>
        }
        icon={<PlaySquare className="h-5 w-5" strokeWidth={1.5} />}
        title={video.title}
      />
      <main
        id="main-content"
        className="min-h-0 flex-1 overflow-y-auto px-6 pb-12 pt-4"
      >
        <div className="mx-auto grid max-w-[1080px] gap-4 lg:grid-cols-[1.5fr_0.5fr]">
          <Card className="overflow-hidden">
            <div className="aspect-video bg-black">
              <video
                controls
                poster={video.thumbnail}
                className="h-full w-full"
                aria-label={video.title}
              >
                <track kind="captions" />
              </video>
            </div>
            <CardContent className="p-5">
              <div className="text-body font-semibold">{video.title}</div>
              <div className="mt-1 text-caption text-fg-3">
                {video.duration} · {video.clipCount} clips
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="p-5 pb-3">
              <CardTitle className="text-body font-semibold">Clips</CardTitle>
            </CardHeader>
            <CardContent className="space-y-1 p-3 pt-1">
              {clips.map((clip) => (
                <button
                  key={clip.id}
                  type="button"
                  onClick={() => onClip(clip.id)}
                  className={cn(
                    "flex w-full items-center gap-2 rounded-control px-2 py-2.5 text-start hover:bg-subtle",
                    clip.id === clipId && "bg-selected",
                  )}
                >
                  <span className="w-10 font-mono text-caption text-fg-3">
                    {clip.start}
                  </span>
                  <span className="min-w-0 flex-1 truncate text-caption text-fg-1">
                    {clip.title}
                  </span>
                  <span className="text-[10px] text-fg-3">{clip.duration}</span>
                </button>
              ))}
            </CardContent>
          </Card>
        </div>
      </main>
    </>
  );
}

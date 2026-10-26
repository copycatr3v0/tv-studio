import { useEffect, useState } from "react";
import { api } from "../lib/ipc";
import { SAMPLE_APPS, toApp, type PreviewApp } from "./apps";

/** The selected TV's launchable apps for the preview, or clearly-labelled sample apps without a TV. */
export function useTvApps(serial: string | null): { apps: PreviewApp[]; source: "tv" | "sample"; packages: string[] } {
  const [pkgs, setPkgs] = useState<string[] | null>(null);
  useEffect(() => {
    setPkgs(null);
    if (!serial) return;
    let live = true;
    api.launchableApps(serial).then((p) => { if (live) setPkgs(p); }, () => { if (live) setPkgs(null); });
    return () => { live = false; };
  }, [serial]);
  if (pkgs && pkgs.length) return { apps: pkgs.map(toApp), source: "tv", packages: pkgs };
  return { apps: SAMPLE_APPS, source: "sample", packages: [] };
}

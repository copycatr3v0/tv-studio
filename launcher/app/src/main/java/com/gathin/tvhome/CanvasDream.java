package com.gathin.tvhome;

import android.service.dreams.DreamService;

/**
 * The canvas as a screensaver: the layout's background (picture, animated picture or video) and its widgets, with
 * no apps and nothing to focus. Without a canvas layout it shows the clock screensaver instead. Studio sets it with
 * {@code settings put secure screensaver_components studio.tvcustomizer.home/com.gathin.tvhome.CanvasDream} and
 * puts the TV's previous screensaver back on Revert.
 */
public class CanvasDream extends DreamService {

    private CanvasHome canvas;
    private ClockScene clock;

    @Override
    public void onAttachedToWindow() {
        super.onAttachedToWindow();
        setInteractive(false);   // any key wakes the TV back to where it was
        setFullscreen(true);
        setScreenBright(false);
        LayoutSpec spec = CanvasHome.load(this);
        if (spec != null) {
            canvas = new CanvasHome(this, spec, Config.load(this).reducedMotion, true);
            setContentView(canvas);
        } else {
            setContentView(R.layout.dream_clock);
            clock = new ClockScene(getWindow().getDecorView());
            clock.useHomeWallpaper(this);
        }
    }

    @Override
    public void onDreamingStarted() {
        super.onDreamingStarted();
        if (canvas != null) canvas.start();
        if (clock != null) clock.start();
    }

    @Override
    public void onDreamingStopped() {
        if (canvas != null) canvas.stop();
        if (clock != null) clock.stop();
        super.onDreamingStopped();
    }

    @Override
    public void onDetachedFromWindow() {
        if (canvas != null) canvas.stop();
        if (clock != null) clock.stop();
        super.onDetachedFromWindow();
    }
}

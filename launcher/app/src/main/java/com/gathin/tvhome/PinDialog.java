package com.gathin.tvhome;

import android.app.Dialog;
import android.content.Context;
import android.graphics.Color;
import android.graphics.drawable.GradientDrawable;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.GridLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

/**
 * A D-pad keypad for the PIN (the remote's number keys work too). With no PIN set yet it asks for a new one twice;
 * otherwise it checks it. {@code onOk} runs only after a correct PIN.
 */
final class PinDialog {
    private PinDialog() {}

    static void ask(Context ctx, String why, int accent, Runnable onOk) {
        final boolean create = !Pin.isSet(ctx);
        Dialog d = new Dialog(ctx, android.R.style.Theme_Translucent_NoTitleBar_Fullscreen);
        float unit = Math.max(0.5f, ctx.getResources().getDisplayMetrics().heightPixels / 1080f);
        LinearLayout box = new LinearLayout(ctx);
        box.setOrientation(LinearLayout.VERTICAL);
        box.setGravity(Gravity.CENTER);
        box.setBackgroundColor(0xE6000000);
        TextView title = text(ctx, create ? "Choose a PIN (4 to 8 digits)" : why, 34 * unit, Color.WHITE);
        TextView dots = text(ctx, " ", 48 * unit, Color.WHITE);
        TextView note = text(ctx, create ? "It stays on this TV. Studio can only reset it." : " ", 22 * unit, 0xAAFFFFFF);
        box.addView(title);
        box.addView(dots);
        box.addView(note);
        final StringBuilder typed = new StringBuilder();
        final String[] first = {null};
        GridLayout pad = new GridLayout(ctx);
        pad.setColumnCount(3);
        String[] keys = {"1", "2", "3", "4", "5", "6", "7", "8", "9", "⌫", "0", "OK"};
        final Runnable[] submit = new Runnable[1];
        final Runnable refresh = () -> {
            StringBuilder s = new StringBuilder();
            for (int i = 0; i < typed.length(); i++) s.append("● ");
            dots.setText(s.length() == 0 ? " " : s.toString().trim());
        };
        for (String k : keys) {
            Button b = new Button(ctx);
            b.setText(k);
            b.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, 30 * unit);
            b.setTextColor(Color.WHITE);
            GradientDrawable bg = new GradientDrawable();
            bg.setCornerRadius(16 * unit);
            bg.setColor(0x33FFFFFF);
            b.setBackground(bg);
            b.setOnFocusChangeListener((v, f) -> { bg.setColor(f ? accent : 0x33FFFFFF); b.setTextColor(f && (accent & 0xFFFFFF) > 0x808080 ? Color.BLACK : Color.WHITE); });
            GridLayout.LayoutParams lp = new GridLayout.LayoutParams();
            lp.width = (int) (120 * unit); lp.height = (int) (90 * unit);
            lp.setMargins((int) (8 * unit), (int) (8 * unit), (int) (8 * unit), (int) (8 * unit));
            pad.addView(b, lp);
            b.setOnClickListener(v -> {
                if ("⌫".equals(k)) { if (typed.length() > 0) typed.setLength(typed.length() - 1); }
                else if ("OK".equals(k)) submit[0].run();
                else if (typed.length() < Pin.MAX_LEN) typed.append(k);
                refresh.run();
            });
        }
        LinearLayout.LayoutParams plp = new LinearLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        plp.topMargin = (int) (24 * unit);
        box.addView(pad, plp);
        submit[0] = () -> {
            String pin = typed.toString();
            typed.setLength(0);
            refresh.run();
            long now = System.currentTimeMillis();
            if (create) {
                if (!Pin.valid(pin)) { note.setText("Use 4 to 8 digits."); return; }
                if (first[0] == null) { first[0] = pin; title.setText("Type the same PIN again"); note.setText(" "); return; }
                if (!first[0].equals(pin)) { first[0] = null; title.setText("They didn't match. Choose a PIN"); return; }
                Pin.set(ctx, pin);
                d.dismiss();
                onOk.run();
                return;
            }
            long wait = Pin.waitLeft(ctx, now);
            if (wait > 0) { note.setText("Too many tries. Wait " + (wait / 1000 + 1) + " s."); return; }
            if (Pin.check(ctx, pin, now)) { d.dismiss(); onOk.run(); }
            else {
                long w = Pin.waitLeft(ctx, now);
                note.setText(w > 0 ? "Too many tries. Wait " + (w / 1000 + 1) + " s." : "Wrong PIN.");
            }
        };
        d.setContentView(box);
        d.setOnKeyListener((dialog, keyCode, e) -> {
            if (e.getAction() != KeyEvent.ACTION_UP) return false;
            if (keyCode >= KeyEvent.KEYCODE_0 && keyCode <= KeyEvent.KEYCODE_9) {
                if (typed.length() < Pin.MAX_LEN) typed.append((char) ('0' + keyCode - KeyEvent.KEYCODE_0));
                refresh.run();
                return true;
            }
            if (keyCode == KeyEvent.KEYCODE_DEL) { if (typed.length() > 0) typed.setLength(typed.length() - 1); refresh.run(); return true; }
            return false;
        });
        d.setOnShowListener(x -> pad.getChildAt(0).requestFocus());
        d.show();
    }

    private static TextView text(Context c, String s, float px, int color) {
        TextView t = new TextView(c);
        t.setText(s);
        t.setTextColor(color);
        t.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, px);
        t.setGravity(Gravity.CENTER);
        t.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_YES);
        return t;
    }
}

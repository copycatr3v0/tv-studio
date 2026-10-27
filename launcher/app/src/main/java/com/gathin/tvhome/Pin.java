package com.gathin.tvhome;

import android.content.Context;
import android.content.SharedPreferences;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.security.SecureRandom;

/**
 * The TV's PIN for locked apps and protected profiles. It is chosen on the TV the first time it is needed and kept
 * only in this app's private storage (salted, stretched hash): Studio never sees it and it never travels over adb.
 * After five wrong tries the keypad waits 30 seconds. Studio can only ask for it to be forgotten (theme file
 * pin_reset), after which the next locked app asks for a new one.
 */
final class Pin {
    private Pin() {}

    static final String PREFS = "pin";
    static final int MIN_LEN = 4, MAX_LEN = 8, MAX_TRIES = 5;
    static final long COOLDOWN_MS = 30_000;
    private static final int ROUNDS = 20_000;

    static boolean valid(String pin) {
        return pin != null && pin.length() >= MIN_LEN && pin.length() <= MAX_LEN && pin.chars().allMatch(Character::isDigit);
    }

    /** Hex SHA-256 of salt and PIN, stretched so a copied hash is slow to guess. */
    static String hash(String saltHex, String pin) {
        try {
            MessageDigest md = MessageDigest.getInstance("SHA-256");
            byte[] h = (saltHex + ":" + pin).getBytes(StandardCharsets.UTF_8);
            for (int i = 0; i < ROUNDS; i++) { md.update(h); md.update(pin.getBytes(StandardCharsets.UTF_8)); h = md.digest(); }
            return hex(h);
        } catch (NoSuchAlgorithmException e) {
            throw new IllegalStateException(e);
        }
    }

    static String hex(byte[] b) {
        StringBuilder sb = new StringBuilder(b.length * 2);
        for (byte x : b) sb.append(String.format("%02x", x & 0xFF));
        return sb.toString();
    }

    private static SharedPreferences prefs(Context c) { return c.getSharedPreferences(PREFS, Context.MODE_PRIVATE); }

    static boolean isSet(Context c) { return prefs(c).contains("hash"); }

    static void set(Context c, String pin) {
        if (!valid(pin)) throw new IllegalArgumentException("pin");
        byte[] salt = new byte[16];
        new SecureRandom().nextBytes(salt);
        String s = hex(salt);
        prefs(c).edit().putString("salt", s).putString("hash", hash(s, pin)).putInt("tries", 0).putLong("until", 0).apply();
    }

    static void forget(Context c) { prefs(c).edit().clear().apply(); }

    /** Milliseconds left before another try is allowed (0 when the keypad is open). */
    static long waitLeft(Context c, long now) { return Math.max(0, prefs(c).getLong("until", 0) - now); }

    /** Checks a PIN, counting wrong tries; false while the keypad is waiting. */
    static boolean check(Context c, String pin, long now) {
        SharedPreferences p = prefs(c);
        if (waitLeft(c, now) > 0 || !valid(pin)) return false;
        boolean ok = hash(p.getString("salt", ""), pin).equals(p.getString("hash", "-"));
        int tries = ok ? 0 : p.getInt("tries", 0) + 1;
        SharedPreferences.Editor e = p.edit().putInt("tries", tries >= MAX_TRIES ? 0 : tries);
        if (tries >= MAX_TRIES) e.putLong("until", now + COOLDOWN_MS);
        e.apply();
        return ok;
    }
}

package com.gathin.tvhome;

import android.content.Context;
import android.content.SharedPreferences;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/**
 * Weather for one city from Open-Meteo (no account, no key). Only when the layout turns it on (the person agreed in
 * Studio that the TV may contact Open-Meteo): the city name is sent once to find it, then its coordinates every
 * 30 minutes while Home is on screen. No location permission, nothing else leaves the TV. The last reading is kept
 * so Home shows it straight away and when the network is down.
 */
final class Weather {
    private Weather() {}

    static final long REFRESH_MS = 30 * 60_000L;
    private static final String PREFS = "weather";
    private static final ExecutorService NET = Executors.newSingleThreadExecutor();

    /** A reading: current temperature and condition, and the next three days. */
    static final class Now {
        double temp;
        int code;
        boolean day = true;
        String city = "";
        final double[] hi = new double[3], lo = new double[3];
        final int[] dayCode = new int[3];
        long at;
    }

    /** WMO weather code to a short word. */
    static String word(int code) {
        if (code == 0) return "Clear";
        if (code <= 2) return "Partly cloudy";
        if (code == 3) return "Cloudy";
        if (code == 45 || code == 48) return "Fog";
        if (code >= 51 && code <= 57) return "Drizzle";
        if ((code >= 61 && code <= 67) || (code >= 80 && code <= 82)) return "Rain";
        if ((code >= 71 && code <= 77) || code == 85 || code == 86) return "Snow";
        if (code >= 95) return "Thunderstorm";
        return "—";
    }

    static String icon(int code, boolean day) {
        if (code == 0) return day ? "☀️" : "🌙";
        if (code <= 2) return day ? "🌤️" : "☁️";
        if (code == 3) return "☁️";
        if (code == 45 || code == 48) return "🌫️";
        if (code >= 51 && code <= 57) return "🌦️";
        if ((code >= 61 && code <= 67) || (code >= 80 && code <= 82)) return "🌧️";
        if ((code >= 71 && code <= 77) || code == 85 || code == 86) return "❄️";
        if (code >= 95) return "⛈️";
        return "☁️";
    }

    /** The scene the weather calls for when the layout follows it: rain or snow, else the layout's own. */
    static String sceneFor(int code, String own) {
        String w = word(code);
        if ("Rain".equals(w) || "Drizzle".equals(w) || "Thunderstorm".equals(w)) return "rain";
        if ("Snow".equals(w)) return "snow";
        return own;
    }

    static String temp(double c, boolean fahrenheit) {
        return Math.round(fahrenheit ? c * 9 / 5 + 32 : c) + "°";
    }

    static Now cached(Context ctx, String city) {
        SharedPreferences p = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        if (!city.equalsIgnoreCase(p.getString("city", ""))) return null;
        try { return parseForecast(new JSONObject(p.getString("json", "{}")), p.getString("label", city), p.getLong("at", 0)); }
        catch (Exception e) { return null; }
    }

    /** Fetch in the background when the cached reading is older than REFRESH_MS; {@code done} runs on any thread. */
    static void refresh(Context ctx, String city, Runnable done) {
        Context app = ctx.getApplicationContext();
        Now c = cached(app, city);
        if (c != null && System.currentTimeMillis() - c.at < REFRESH_MS) return;
        NET.execute(() -> {
            try {
                SharedPreferences p = app.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
                double lat, lon;
                String label;
                if (city.equalsIgnoreCase(p.getString("city", "")) && p.contains("lat")) {
                    lat = Double.longBitsToDouble(p.getLong("lat", 0));
                    lon = Double.longBitsToDouble(p.getLong("lon", 0));
                    label = p.getString("label", city);
                } else {
                    JSONObject g = new JSONObject(get("https://geocoding-api.open-meteo.com/v1/search?count=1&name="
                            + URLEncoder.encode(city, "UTF-8")));
                    JSONArray r = g.optJSONArray("results");
                    if (r == null || r.length() == 0) return;
                    JSONObject first = r.getJSONObject(0);
                    lat = first.getDouble("latitude");
                    lon = first.getDouble("longitude");
                    label = first.optString("name", city);
                }
                String json = get("https://api.open-meteo.com/v1/forecast?latitude=" + lat + "&longitude=" + lon
                        + "&current=temperature_2m,weather_code,is_day&daily=weather_code,temperature_2m_max,temperature_2m_min"
                        + "&timezone=auto&forecast_days=3");
                parseForecast(new JSONObject(json), label, 0);   // refuse anything that does not parse
                p.edit().putString("city", city).putString("label", label).putLong("lat", Double.doubleToLongBits(lat))
                        .putLong("lon", Double.doubleToLongBits(lon)).putString("json", json).putLong("at", System.currentTimeMillis()).apply();
                done.run();
            } catch (Exception e) {
                // offline or the service is down: the last reading stays
            }
        });
    }

    static Now parseForecast(JSONObject o, String label, long at) throws org.json.JSONException {
        Now n = new Now();
        JSONObject cur = o.getJSONObject("current");
        n.temp = cur.getDouble("temperature_2m");
        n.code = cur.getInt("weather_code");
        n.day = cur.optInt("is_day", 1) == 1;
        n.city = label;
        n.at = at;
        JSONObject d = o.optJSONObject("daily");
        if (d != null) {
            JSONArray hi = d.getJSONArray("temperature_2m_max"), lo = d.getJSONArray("temperature_2m_min"), codes = d.getJSONArray("weather_code");
            for (int i = 0; i < 3 && i < hi.length(); i++) { n.hi[i] = hi.getDouble(i); n.lo[i] = lo.getDouble(i); n.dayCode[i] = codes.getInt(i); }
        }
        return n;
    }

    private static String get(String url) throws java.io.IOException {
        HttpURLConnection c = (HttpURLConnection) new URL(url).openConnection();
        c.setConnectTimeout(8000);
        c.setReadTimeout(8000);
        c.setRequestProperty("User-Agent", "StudioHome");
        try (InputStream in = c.getInputStream()) {
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            byte[] buf = new byte[8192];
            int n, total = 0;
            while ((n = in.read(buf)) > 0) {
                total += n;
                if (total > 256 * 1024) throw new java.io.IOException("too large");
                out.write(buf, 0, n);
            }
            return new String(out.toByteArray(), StandardCharsets.UTF_8);
        } finally {
            c.disconnect();
        }
    }
}

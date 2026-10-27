package com.gathin.tvhome;

import org.json.JSONObject;
import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;

public class WeatherTest {
    @Test public void codesWordsAndScenes() {
        assertEquals("Clear", Weather.word(0));
        assertEquals("Rain", Weather.word(63));
        assertEquals("Snow", Weather.word(73));
        assertEquals("Thunderstorm", Weather.word(95));
        assertEquals("rain", Weather.sceneFor(61, "aurora"));
        assertEquals("snow", Weather.sceneFor(75, "aurora"));
        assertEquals("aurora", Weather.sceneFor(1, "aurora"));
    }

    @Test public void temperatures() {
        assertEquals("21°", Weather.temp(21.4, false));
        assertEquals("71°", Weather.temp(21.4, true));
    }

    @Test public void parsesOpenMeteo() throws Exception {
        String json = "{\"current\":{\"temperature_2m\":28.6,\"weather_code\":61,\"is_day\":0},"
                + "\"daily\":{\"temperature_2m_max\":[31,32,30],\"temperature_2m_min\":[24,25,23],\"weather_code\":[61,2,0]}}";
        Weather.Now n = Weather.parseForecast(new JSONObject(json), "Ongole", 5);
        assertEquals(28.6, n.temp, 0.01);
        assertEquals(61, n.code);
        assertFalse(n.day);
        assertEquals(32, n.hi[1], 0.01);
        assertEquals(0, n.dayCode[2]);
        assertEquals("Ongole", n.city);
    }
}

package com.gathin.tvhome;

import android.service.notification.NotificationListenerService;

/**
 * Exists only so the Now Playing widget may read which media is playing (MediaSessionManager.getActiveSessions
 * needs an enabled notification listener). It reads no notifications. Studio switches it on with
 * {@code cmd notification allow_listener} while a layout uses the widget, and off again otherwise and on Revert.
 */
public class MediaListener extends NotificationListenerService {
}

import java.nio.file.Paths;

import jdk.jfr.consumer.RecordedEvent;
import jdk.jfr.consumer.RecordedThread;
import jdk.jfr.consumer.RecordingFile;

public class Example {
    public static void main(String[] args) throws Exception {
        int iteration = Integer.parseInt(args[1]);
        for (int i = 0; i < iteration; i++) {
            try (RecordingFile jfr = new RecordingFile(Paths.get(args[0]))) {
                System.out.println("started");

                int eventCount = 0;
                int totalOsNameLength = 0;
                while (jfr.hasMoreEvents()) {
                    RecordedEvent event = jfr.readEvent();
                    if (!"jdk.ExecutionSample".equals(event.getEventType().getName())) {
                        continue;
                    }
                    RecordedThread thread = event.getValue("sampledThread");
                    totalOsNameLength += thread.getOSName().length();
                    eventCount++;
                }

                System.out.printf("event count: %d, os name length: %d\n", eventCount, totalOsNameLength);
            }
        }
    }
}

import java.io.IOException;
import java.io.PrintWriter;
import java.io.UncheckedIOException;
import java.nio.ByteBuffer;
import java.nio.channels.FileChannel;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardOpenOption;
import java.util.Random;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

public class Example {
    public static void main(String[] args) throws Exception {
        Path baseDir = Paths.get(args[0]);
        CountDownLatch terminationLatch = new CountDownLatch(1);
        Runtime.getRuntime().addShutdownHook(new Thread(terminationLatch::countDown));

        int gThreadCount = 2;
        int wThreadCount = 2;

        AtomicInteger gThreadId = new AtomicInteger(0);
        ExecutorService generators = Executors.newFixedThreadPool(gThreadCount, r -> {
            Thread th = new Thread(r);
            th.setName("generator-" + gThreadId.getAndIncrement());
            return th;
        });

        AtomicInteger wgThreadId = new AtomicInteger(0);
        ExecutorService writers = Executors.newFixedThreadPool(wThreadCount, r -> {
            Thread th = new Thread(r);
            th.setName("writer-" + wgThreadId.getAndIncrement());
            return th;
        });

        LinkedBlockingQueue<String> messageQueue = new LinkedBlockingQueue<>();

        for (int i = 0; i < wThreadCount; i++) {
            int n = i;
            writers.execute(() -> {
                Path out = baseDir.resolve("out" + n);
                try (Writer writer = new Writer(out)) {
                    while (terminationLatch.getCount() > 0) {
                        String message = messageQueue.poll(100L, TimeUnit.MILLISECONDS);
                        if (message != null) {
                            writer.println(message);
                        }
                    }
                } catch (InterruptedException e) {
                    Thread.currentThread().interrupt();
                    throw new RuntimeException(e);
                }
            });
        }

        for (int i = 0; i < gThreadCount; i++) {
            generators.execute(() -> {
                MessageGenerator messageGen = new MessageGenerator();
                while (terminationLatch.getCount() > 0) {
                    try {
                        messageQueue.put(messageGen.nextMessage());
                        sleep(500L);
                    } catch (InterruptedException e) {
                        Thread.currentThread().interrupt();
                        throw new RuntimeException(e);
                    }
                }
            });
        }

        generators.shutdown();
        writers.shutdown();

        generators.awaitTermination(Long.MAX_VALUE, TimeUnit.MILLISECONDS);
        writers.awaitTermination(Long.MAX_VALUE, TimeUnit.MILLISECONDS);
    }

    private static void sleep(long millis) {
        long t0 = System.nanoTime();
        while (true) {
            if (System.nanoTime() - t0 >= millis * 1000 * 1000) {
                break;
            }
            Thread.yield();
        }
    }

    static class Writer implements AutoCloseable {
        private final FileChannel channel;

        public Writer(Path path) {
            try {
                channel = FileChannel.open(path, StandardOpenOption.CREATE, StandardOpenOption.WRITE);
            } catch (IOException e) {
                throw new UncheckedIOException(e);
            }
        }

        public void println(String message) {
            ByteBuffer buf = ByteBuffer.allocate(message.length());
            try {
                channel.write(buf);
                channel.force(true);
            } catch (IOException e) {
                throw new UncheckedIOException(e);
            }
        }

        @Override
        public void close() {
            try {
                channel.close();
            } catch (IOException e) {
                throw new UncheckedIOException(e);
            }
        }
    }

    static class MessageGenerator {
        private final Random rnd = new Random();

        public String nextMessage() {
            return "message:" + rnd.nextInt();
        }
    }
}

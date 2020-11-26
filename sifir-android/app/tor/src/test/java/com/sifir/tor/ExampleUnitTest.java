package com.sifir.tor;

import org.junit.Test;
/**
 * Example local unit test, which will execute on the development machine (host).
 *
 * @see <a href="http://d.android.com/tools/testing">Testing documentation</a>
 */
public class ExampleUnitTest {
    static {
        try {
             System.load("/home/gus/Projects/sifir-io-public/sifir-rs-sdk/sifir-android/app/tor/src/test/jniLibs/x86_64/libsifir_android.so");
//        	System.loadLibrary("libsifir_android.so");
        } catch (UnsatisfiedLinkError e) {
            System.err.println("Native code library failed to load.\n" + e);
            System.exit(1);
        }
    }

    @Test
    public void can_start_tor()
    {
        TorServiceParam param = new TorServiceParam("/tmp/javajtor",19002);
        OwnedTorService service = new OwnedTorService(param);
        service.shutdown();
    }
}

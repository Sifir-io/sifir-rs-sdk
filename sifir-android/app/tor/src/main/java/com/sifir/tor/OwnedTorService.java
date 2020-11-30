// Automatically generated by flapigen
package com.sifir.tor;


public final class OwnedTorService {

    public OwnedTorService(TorServiceParam param) {
        long a0 = param.mNativeObj;
        param.mNativeObj = 0;

        mNativeObj = init(a0);
        JNIReachabilityFence.reachabilityFence1(param);
    }
    private static native long init(long param);

    public final int getSocksPort() {
        int ret = do_getSocksPort(mNativeObj);

        return ret;
    }
    private static native int do_getSocksPort(long self);

    public final void shutdown() {
        do_shutdown(mNativeObj);
    }
    private static native void do_shutdown(long self);

    public final String get_status() {
        String ret = do_get_status(mNativeObj);

        return ret;
    }
    private static native String do_get_status(long self);

    public synchronized void delete() {
        if (mNativeObj != 0) {
            do_delete(mNativeObj);
            mNativeObj = 0;
       }
    }
    @Override
    protected void finalize() throws Throwable {
        try {
            delete();
        }
        finally {
             super.finalize();
        }
    }
    private static native void do_delete(long me);
    /*package*/ OwnedTorService(InternalPointerMarker marker, long ptr) {
        assert marker == InternalPointerMarker.RAW_PTR;
        this.mNativeObj = ptr;
    }
    /*package*/ long mNativeObj;
}
import faulthandler, os, signal, resource
resource.setrlimit(resource.RLIMIT_CORE,(0,0))
faulthandler.enable(all_threads=True)
def synthetic_signal_only():
    os.kill(os.getpid(),signal.SIGSEGV)
synthetic_signal_only()

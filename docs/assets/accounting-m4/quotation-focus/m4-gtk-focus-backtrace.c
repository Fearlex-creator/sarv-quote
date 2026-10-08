#define _GNU_SOURCE
#include <dlfcn.h>
#include <execinfo.h>
#include <stdio.h>
void g_return_if_fail_warning(const char *domain,const char *function,const char *expression) {
    void *frames[80];
    fprintf(stderr,"\nM4 DIAGNOSTIC RETURN CHECK: %s / %s / %s\n",domain?domain:"",function?function:"",expression?expression:"");
    int count=backtrace(frames,80);
    backtrace_symbols_fd(frames,count,2);
    void (*original)(const char *,const char *,const char *)=dlsym(RTLD_NEXT,"g_return_if_fail_warning");
    if(original)original(domain,function,expression);
}

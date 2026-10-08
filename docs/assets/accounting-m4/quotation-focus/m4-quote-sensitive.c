#define _GNU_SOURCE
#include <dlfcn.h>
#include <execinfo.h>
#include <stdio.h>
#include <time.h>
static void *focused(void *w) {
 int (*has)(void*)=dlsym(RTLD_NEXT,"gtk_widget_has_focus");void *(*first)(void*)=dlsym(RTLD_NEXT,"gtk_widget_get_first_child");void *(*next)(void*)=dlsym(RTLD_NEXT,"gtk_widget_get_next_sibling");
 if(has(w))return w; for(void *c=first(w);c;c=next(c)){void *f=focused(c);if(f)return f;}return 0;
}
void gtk_widget_set_sensitive(void *w,int sensitive) {
 if(!sensitive){void *f=focused(w); if(f){
 const char *(*type)(void*)=dlsym(RTLD_NEXT,"g_type_name_from_instance");
 fprintf(stderr,"M4 FOCUSED DISABLE parent=%p/%s focused=%p/%s time=%ld\n",w,type(w),f,type(f),(long)time(0));
 void *frames[60];int n=backtrace(frames,60);backtrace_symbols_fd(frames,n,2);
 }}
 void (*orig)(void*,int)=dlsym(RTLD_NEXT,"gtk_widget_set_sensitive");orig(w,sensitive);
}

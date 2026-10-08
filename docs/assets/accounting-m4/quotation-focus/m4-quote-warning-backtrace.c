#define _GNU_SOURCE
#include <dlfcn.h>
#include <execinfo.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
static void trace(const char *what) { void *frames[70]; fprintf(stderr,"\nM4 QUOTE DIAGNOSTIC %s\n",what); int n=backtrace(frames,70); backtrace_symbols_fd(frames,n,2); }
void g_log(const char *domain,unsigned level,const char *format,...) {
 if(format && (strstr(format,"focus-out") || strstr(format,"requested") || strstr(format,"window only"))) trace(format);
 va_list args; va_start(args,format); void (*orig)(const char*,unsigned,const char*,va_list)=dlsym(RTLD_NEXT,"g_logv"); orig(domain,level,format,args); va_end(args);
}
void gtk_window_set_focus(void *window,void *focus) {
 void *(*get)(void*)=dlsym(RTLD_NEXT,"gtk_window_get_focus");
 const char *(*type)(void*)=dlsym(RTLD_NEXT,"g_type_name_from_instance");
 void *old=get(window);
 fprintf(stderr,"M4 QUOTE FOCUS old=%p/%s new=%p/%s\n",old,old?type(old):"none",focus,focus?type(focus):"none");
 trace("APPLICATION WINDOW SET FOCUS");
 void (*orig)(void*,void*)=dlsym(RTLD_NEXT,"gtk_window_set_focus"); orig(window,focus);
}
void gtk_widget_unparent(void *widget) {
 void *(*root)(void*)=dlsym(RTLD_NEXT,"gtk_widget_get_root");
 int (*isa)(void*,unsigned long)=dlsym(RTLD_NEXT,"g_type_check_instance_is_a");
 unsigned long (*wtype)(void)=dlsym(RTLD_NEXT,"gtk_window_get_type");
 void *(*get)(void*)=dlsym(RTLD_NEXT,"gtk_window_get_focus");
 int (*ancestor)(void*,void*)=dlsym(RTLD_NEXT,"gtk_widget_is_ancestor");
 void *window=root(widget); if(window && isa(window,wtype())) {void *f=get(window); if(f && (f==widget || ancestor(f,widget))) trace("FOCUSED SUBTREE UNPARENT");}
 void (*orig)(void*)=dlsym(RTLD_NEXT,"gtk_widget_unparent"); orig(widget);
}
void g_log_structured_standard(const char *domain,unsigned level,const char *file,const char *line,const char *func,const char *format,...) {
 char message[8192]; va_list args; va_start(args,format); vsnprintf(message,sizeof message,format,args); va_end(args);
 if(strstr(message,"focus-out") || strstr(message,"requested")) {fprintf(stderr,"M4 QUOTE WARNING SITE %s:%s %s\n",file,line,func); trace(message);}
 void (*orig)(const char*,unsigned,const char*,const char*,const char*,const char*,...)=dlsym(RTLD_NEXT,"g_log_structured_standard"); orig(domain,level,file,line,func,"%s",message);
}

#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
static void walk(void *w,int depth) {
 const char *(*type)(void*)=dlsym(RTLD_NEXT,"g_type_name_from_instance"); const char *(*name)(void*)=dlsym(RTLD_NEXT,"gtk_widget_get_name");
 void (*measure)(void*,int,int,int*,int*,int*,int*)=dlsym(RTLD_NEXT,"gtk_widget_measure");
 void *(*first)(void*)=dlsym(RTLD_NEXT,"gtk_widget_get_first_child"); void *(*next)(void*)=dlsym(RTLD_NEXT,"gtk_widget_get_next_sibling");
 int min=0,nat=0; measure(w,1,-1,&min,&nat,0,0); if(min>=350 || depth<3) fprintf(stderr,"M4 HEIGHT depth=%d %s %s min=%d natural=%d\n",depth,type(w),name(w),min,nat);
 if(depth<15) for(void *c=first(w);c;c=next(c)) walk(c,depth+1);
}
static int dump(void *w) {walk(w,0);return 0;}
void gtk_window_present(void *w) {
 void (*orig)(void*)=dlsym(RTLD_NEXT,"gtk_window_present");orig(w);
 unsigned (*later)(unsigned,int(*)(void*),void*)=dlsym(RTLD_NEXT,"g_timeout_add"); later(1500,dump,w);
}

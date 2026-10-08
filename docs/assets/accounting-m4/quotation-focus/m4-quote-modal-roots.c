#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
static const char *typename(void *w) {const char *(*f)(void*)=dlsym(RTLD_NEXT,"g_type_name_from_instance"); return w?f(w):"none";}
void adw_dialog_present(void *dialog,void *parent) {
 void *(*visible)(void*)=dlsym(RTLD_NEXT,"adw_application_window_get_visible_dialog");void *(*focus)(void*)=dlsym(RTLD_NEXT,"adw_dialog_get_focus"); void *(*root)(void*)=dlsym(RTLD_NEXT,"gtk_widget_get_root");
 void *v=visible(parent);void *f=v?focus(v):0;
 fprintf(stderr,"M4 MODAL PRESENT parent=%p/%s visible=%p/%s focus=%p/%s root=%p/%s\n",parent,typename(parent),v,typename(v),f,typename(f),f?root(f):0,typename(f?root(f):0));
 void (*orig)(void*,void*)=dlsym(RTLD_NEXT,"adw_dialog_present");orig(dialog,parent);
}

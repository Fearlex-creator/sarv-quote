#define _GNU_SOURCE
#include <pthread.h>
#include <dlfcn.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <stdio.h>
#include <time.h>
#include <sys/syscall.h>
static double ms(void){struct timespec t;clock_gettime(CLOCK_MONOTONIC,&t);return t.tv_sec*1000.+t.tv_nsec/1000000.;}
int pthread_setname_np(pthread_t thread,const char *name){
 int (*real_fn)(pthread_t,const char*)=dlsym(RTLD_NEXT,"pthread_setname_np");
 if(!real_fn)return 38;
 int rc=real_fn(thread,name);
 const char *flag=getenv("SARV_QA_WORKER_DELAY_FLAG"),*trace=getenv("SARV_QA_WORKER_DELAY_TRACE");
 if(rc==0&&pthread_equal(thread,pthread_self())&&strcmp(name,"sarv-export")==0&&flag&&access(flag,F_OK)==0){
  double start=ms();long tid=syscall(SYS_gettid);
  if(trace){FILE *f=fopen(trace,"a");if(f){fprintf(f,"start tid=%ld at_ms=%.3f\n",tid,start);fclose(f);}}
  while(access(flag,F_OK)==0&&ms()-start<5000.)usleep(1000);
  double held=ms()-start;int timeout=access(flag,F_OK)==0;
  if(trace){FILE *f=fopen(trace,"a");if(f){fprintf(f,"end tid=%ld held_ms=%.3f timeout=%d\n",tid,held,timeout);fclose(f);}}
 }
 return rc;
}

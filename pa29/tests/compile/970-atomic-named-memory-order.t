enum { relaxed_order=0, acquire_order=2, release_order=3 };
const int named_relaxed=0;
void named(int* p,int value){
  __atomic_store_n(p,value,relaxed_order);
  __atomic_store_n(p,value,release_order);
  __atomic_store_n(p,value,named_relaxed);
}
int load(int* p){return __atomic_load_n(p,acquire_order);}
void literal(int* p,int value){__atomic_store_n(p,value,0);}
void dynamic(int* p,int value,int order){__atomic_store_n(p,value,order);}

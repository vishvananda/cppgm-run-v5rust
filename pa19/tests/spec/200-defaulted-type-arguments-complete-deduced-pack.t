// N3485 [temp.arg]/4 and [temp.deduct.type]/9: all defaults belong to the actual type.
struct marker {};
template<class A=marker,class B=marker,class C=marker,class D=marker>struct box{};
template<class,class>struct same;
template<class T>struct same<T,T>{};
same<box<int>,box<int,marker,marker,marker>> identity;
template<class T,class...Ts>constexpr int count(const box<T,Ts...>&){return sizeof...(Ts);}
template<class T,class...Ts>int paired(const box<T,Ts...>&,const box<T,Ts...>&){return sizeof...(Ts);}
template<class T,class...Ts>int pointed(const box<T*,Ts*...>&){return sizeof...(Ts);}
template<class T>using alias=box<T>;
struct derived:box<int,int>{};
static_assert(count(box<>())==3,"all defaults");
static_assert(count(box<int,int>())==3,"partial defaults");
static_assert(count(box<int,int,marker,marker>())==3,"explicit defaults");
static_assert(count(box<int,int,long,char>())==3,"different trailing types");
static_assert(count(derived())==3,"inherited type");
static_assert(count(alias<int>())==3,"alias type");
// Taking the address also retains the written box<T,Ts...> ABI pattern.
int (*paired_address)(const box<int>&,const box<int>&)=&paired<int,marker,marker,marker>;
int (*pointed_address)(const box<int*,char*,long*,short*>&)=&pointed<int,char,long,short>;
int main(){return paired(box<int>(),box<int,marker,marker,marker>())!=3 ||
  paired_address(box<int>(),box<int>())!=3 ||
  pointed_address(box<int*,char*,long*,short*>())!=3;}

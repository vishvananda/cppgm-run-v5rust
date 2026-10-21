// N3485 5.2.7p9, 5.2.8p2, 12.2p3 and 15.2p1: RTTI failure
// destroys evaluated temporaries and lexical objects; expect exit 0
namespace std {class type_info;}
int live,calls;
struct G {G(){++live;} ~G()noexcept(false){--live;throw 7;}};
struct B {virtual int key(){return 1;}};
struct D:B {};
B& use(G const&,B& b){++calls;return b;}
B* ptr(G const&,B* b){++calls;return b;}
int run(D& d,int choice,int op){try{if(op)typeid(*(choice?ptr(G(),&d):ptr(G(),&d)));else dynamic_cast<D&>(choice?use(G(),d):use(G(),d));return 1;}catch(int x){return x!=7 || live || calls!=1;}catch(...){return 2;}}
int main(){D d;for(int i=0;i!=2;++i)for(int op=0;op!=2;++op){calls=0;if(run(d,i,op))return 3;}return 0;}

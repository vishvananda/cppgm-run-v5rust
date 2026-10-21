// N3485 5.2.7p9, 5.2.8p2, 12.2p3 and 15.2p1: RTTI failure
// destroys evaluated temporaries and lexical objects; expect exit 0
namespace std {class type_info;}
int live,calls;
struct G {G(){++live;} ~G(){--live;}};
struct B {virtual int key(){return 1;}};
struct D:B {};
B& use(G const&,G const&,B& b){++calls;return b;}
B* ptr(G const&,G const&,B* b){++calls;return b;}
int run(B& b,int choice,int op){G outer;try{if(op)typeid(*(choice?ptr(G(),G(),0):ptr(G(),G(),0)));else dynamic_cast<D&>(choice?use(G(),G(),b):use(G(),G(),b));return 1;}catch(...){return live!=1 || calls!=1;}}
int main(){B b;for(int i=0;i!=2;++i)for(int op=0;op!=2;++op){calls=0;if(run(b,i,op) || live)return 2;}return 0;}

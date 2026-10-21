// N3485 5.2.7p9, 5.2.8p2, 12.2p3 and 15.2p1: RTTI failure
// destroys evaluated temporaries and lexical objects; expect exit 0
namespace std {class type_info;}
int live,calls,handled,bad;
struct Error {Error(){++handled;} Error(Error const&){++handled;} ~Error(){--handled;}};
struct G {G(){++live;} ~G(){if(handled!=1)bad=1;--live;}};
struct B {virtual int key(){return 1;}};
struct D:B {};
B& use(G const&,B& b){++calls;return b;}
B* ptr(G const&,B* b){++calls;return b;}
int run(int choice,int op){B b;try{throw Error();}catch(...){G outer;
try{if(op)typeid(*(choice?ptr(G(),0):ptr(G(),0)));else dynamic_cast<D&>(choice?use(G(),b):use(G(),b));return 1;}
catch(...){if(handled!=1 || live!=1 || calls!=1 || bad)return 2;}
}return live || handled || bad;}
int main(){for(int i=0;i!=2;++i)for(int op=0;op!=2;++op){calls=0;if(run(i,op))return 3;}return 0;}

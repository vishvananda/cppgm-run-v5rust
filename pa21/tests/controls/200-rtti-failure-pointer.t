// N3485 5.2.7p9, 5.2.8p2, 12.2p3 and 15.2p1: RTTI failure
// destroys evaluated temporaries and lexical objects; expect exit 0
namespace std {class type_info;}
int live,calls,bad;
struct G {G(){++live;} ~G(){--live;}};
struct B {virtual int key(){return 1;}};
struct D:B {};
B* ptr(G const&,B* b){++calls;return b;}
int consume(D* p){if(live!=1)bad=1;return p!=0;}
int run(B* b,int choice){try{if(consume(dynamic_cast<D*>(choice?ptr(G(),b):ptr(G(),b))))return 1;}catch(...){return 2;}return live || calls!=1 || bad;}
int main(){B b;for(int i=0;i!=2;++i){calls=0;if(run(&b,i))return 1;calls=0;if(run(0,i))return 2;}return 0;}

// N3485 5.2.7p9, 5.2.8p2, 12.2p3 and 15.2p1: RTTI failure
// destroys evaluated temporaries and lexical objects; expect exit 0
namespace std {class type_info;}
int live,calls,bad;
struct G { G(){++live;} ~G(){--live;} };
struct B {virtual int key(){return 1;}};
struct D:B {};
B& use(G const&,B& b){++calls;return b;}
B* ptr(G const&,B* b){++calls;return b;}
void consume(D&){if(live!=1)bad=1;throw 7;}
int later(){if(live!=1)bad=1;throw 7;}
int cast(B& b,int choice){try{consume(dynamic_cast<D&>(choice?use(G(),b):use(G(),b)));return 1;}catch(int x){return x!=7 || live || calls!=1 || bad;}}
int query(B* b,int choice){try{bool n=(&typeid(*(choice?ptr(G(),b):ptr(G(),b)))!=0) && later();return 2+n;}catch(int x){return x!=7 || live || calls!=1 || bad;}}
int main(){D d;for(int i=0;i!=2;++i){calls=0;if(cast(d,i))return 1;calls=0;if(query(&d,i))return 2;}return 0;}

// N3485 5.2.7p9, 5.2.8p2, 12.2p3 and 15.2p1: RTTI failure
// destroys evaluated temporaries and lexical objects; expect exit 0
int live,calls;
struct G {G(){++live;} ~G(){--live;}};
struct B {virtual int key(){return 1;}};
struct D:B {};
B&& use(G const&,B& b){++calls;return static_cast<B&&>(b);}
int run(B& b,int choice){try{dynamic_cast<D&&>(choice?use(G(),b):use(G(),b));}catch(...){return live || calls!=1;}return 2;}
int main(){B b;for(int i=0;i!=2;++i){calls=0;if(run(b,i))return 1;}return 0;}

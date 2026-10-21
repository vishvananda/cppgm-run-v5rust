// N3485 5.2.7p9, 5.2.8p2, 12.2p3 and 15.2p1: RTTI failure
// destroys evaluated temporaries and lexical objects; expect exit 0
int live, calls;
struct G { G(){++live;} ~G(){--live;} };
struct B { virtual int key(){return 1;} };
struct D:B {};
B& use(G const&, B& b){++calls;return b;}
int probe(B& b, int choice){try{dynamic_cast<D&>(choice ? use(G(),b) : use(G(),b));return 1;}catch(...){return live || calls!=1 ? 2:0;}}
int main(){B b;for(int i=0;i!=2;++i){calls=0;int n=probe(b,i);if(n)return n;}return 0;}

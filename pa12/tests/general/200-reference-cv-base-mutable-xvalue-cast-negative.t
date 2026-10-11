// C++11 static_cast/reference binding: N3485 5.2.9 and 8.5.3.
struct B{int value;};struct D:B{};
int main(){D d;d.value=3;B& r=static_cast<B&>(static_cast<D&&>(d));return r.value;}

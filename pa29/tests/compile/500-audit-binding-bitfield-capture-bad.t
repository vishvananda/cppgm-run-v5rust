struct S{signed int a:3;unsigned int b:3;};int f(){S s={-2,5};auto&[a,b]=s;return [=](){return a+b;}();}

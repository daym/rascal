program b;
uses a;
var r : TRec;
    x : TFoo;
begin
  r.state := busy;
  x := nil;
  x.f := cb;
end.

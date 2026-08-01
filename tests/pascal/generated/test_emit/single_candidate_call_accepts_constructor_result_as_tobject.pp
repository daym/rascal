unit u;
interface
type
  tnode = class
    constructor create(n : longint);
    constructor make;
  end;
procedure take(o : tobject);
procedure run;
implementation
constructor tnode.create(n : longint); begin end;
constructor tnode.make; begin end;
procedure take(o : tobject); begin end;
procedure run;
begin
  take(tnode.create(1));
  take(tnode.make);
end;
end.

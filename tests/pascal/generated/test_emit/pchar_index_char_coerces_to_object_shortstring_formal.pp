unit u;
interface
type
  tlist = object
    procedure asmwrite(const s : string);
    procedure run(p : pchar; i : longint);
  end;
implementation
procedure tlist.asmwrite(const s : string);
begin
end;
procedure tlist.run(p : pchar; i : longint);
begin
  asmwrite(p[i]);
end;
end.

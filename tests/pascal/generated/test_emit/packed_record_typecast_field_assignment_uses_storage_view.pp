unit u;
interface
type
  tkind = (ka, kb);
  tview = packed record
    lo : word;
    sub : byte;
    kind : tkind;
  end;
procedure run(var r : longint; k : tkind);
implementation
procedure run(var r : longint; k : tkind);
begin
  tview(r).kind := k;
end;
end.

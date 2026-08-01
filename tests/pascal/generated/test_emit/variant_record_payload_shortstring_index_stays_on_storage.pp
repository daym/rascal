unit u;
interface
type
  tview = record
    case tag : longint of
      0 : (text : string[10]);
      1 : (other : longint);
  end;
procedure raw(var x);
procedure take(var ch : char);
procedure run(var view : tview; i : longint; ch : char; var p : pchar);
implementation
procedure raw(var x);
begin
end;
procedure take(var ch : char);
begin
end;
procedure run(var view : tview; i : longint; ch : char; var p : pchar);
begin
  ch := view.text[i];
  view.text[i] := ch;
  inc(view.text[i]);
  p := @view.text[i];
  raw(view.text[i]);
  take(view.text[i]);
end;
end.

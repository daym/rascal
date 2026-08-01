begin
  begin
{$ifdef arm}
    if c<>'$' then
      begin
        asmgetchar:='{';
        exit;
      end
    else
{$endif arm}
      skipcomment;
  end;
end.

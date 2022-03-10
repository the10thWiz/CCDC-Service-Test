/*
 * index.js
 * Copyright (C) 2022 matthew <matthew@WINDOWS-05HIC4F>
 *
 * Distributed under terms of the MIT license.
 */
(function(){
  'use strict';
  
  $.getJSON('/api/status').then((res) => {
    console.log(res);
    let service = '<td>Serivce</td>';
    let stat = '<td>Status</td>';
    for(let a in res) {
      service += '<td>' + a + '</td>';
      if(res[a].up) {
        stat += '<td><div class="alert alert-success">Up</div></td>';
      } else {
        stat += '<td><div class="alert alert-danger">Down: ' + res[a].failure_reason + '</div></td>';
      }
    }
    $('#status_row').html(stat);
    $('#service_row').html(service);
  });
})();

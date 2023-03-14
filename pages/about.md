---
layout: page
title: About
permalink: /about/
weight: 3
---

# **About Me**

Hi I am **{{ site.author.name }}** :wave:,<br>
I am a **computer engineering** student who will be graduating in Q1 2023 with a passion for **game development** and entertainment technology. I love coding in **C++**, **C#**, and **Rust** for game development and **graphics programming** related subjects. Additionally, I am interested in **machine learning** and **backend** development, as I have work experience in those areas. I participate in the Global Game Jam in my country ~~almost~~ every year. I am particularly interested in **engine**, **graphics**, and **gameplay programming**. I am always happy to learn and share the projects I am working on!

<div class="row">
{% include about/skills.html title="Programming Languages" source=site.data.programming-skills %}
{% include about/skills.html title="Backend Skills" source=site.data.backend-skills %}
</div>
<div class="row">
{% include about/skills.html title="Game Development" source=site.data.game-development-skills %}
{% include about/skills.html title="Languages" source=site.data.languages %}
</div>

# **Experience**
<div class="row">
{% include about/timeline.html %}
</div>